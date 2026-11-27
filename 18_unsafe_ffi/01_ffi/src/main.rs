/*
[主题]: FFI 心智 —— C ABI、`extern`、CString 与安全封装
[难度]: Medium
[前置]: `00_unsafe` 本阶段、`05_cargo_modules`（模块）、`03_error_collections`

一、主题说明
- **FFI（Foreign Function Interface）**：Rust 与 C（或其他 C ABI 语言）在同一进程内通过**符号名 + 调用约定**互相调用。
- `extern "C"` 使用 C 的调用约定；`#[no_mangle]` 让链接器看到稳定符号名（如 `my_add`）。
- Rust 调 C：`extern "C" { fn foo(...); }` + `unsafe { foo() }`（因无法保证 C 侧契约）。
- 字符串：**Rust `&str` 不是 C 字符串**；跨越边界用 `CString`（以 `\0` 结尾、拥有分配）与 `CStr`（借用的 `\0` 结尾视图）。
- **Safe wrapper 心法**：`unsafe fn raw_*` 藏在模块内；对外 `fn safe_* -> Result<_, E>`，校验指针/长度/NUL。

二、写法对比
1. 直观版: 同 crate 内 `#[no_mangle] pub extern "C" fn` 模拟 C 库，Rust 用 `extern "C"` 声明后 `unsafe` 调用（理解链接符号）。
2. 进阶版: `SafeCString` + `strlen_via_libc`，先构造 `CString`，再只在「已知 NUL 结尾」时把指针交给 C。

三、真实用途
- 调系统库（libc）、OpenSSL、SQLite C API、硬件 SDK；Rust 写高性能核心 + C 插件互操作。
- 勿把「能调通」当「安全」：线程、错误码、谁 `free` 内存，都要在 wrapper 里写清。
资料: https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html
      https://doc.rust-lang.org/nomicon/ffi.html
      https://doc.rust-lang.org/reference/items/external-blocks.html
      https://doc.rust-lang.org/nomicon/
*/

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ---------------------------------------------------------------------------
// 模拟「C 静态库」：真实项目里这些符号来自 `.a` / `.so`，此处打进同一可执行文件。
// 放在子模块是为了避免「同模块内既有定义又有 extern 声明」的命名冲突；
// 链接器仍会把 `#[no_mangle]` 符号导出给下面的 `extern "C" { ... }` 使用。
// ---------------------------------------------------------------------------
#[allow(dead_code)] // 符号由链接器导出，供下方 `extern "C"` 声明调用
mod fake_c_library {
    use std::os::raw::c_char;

    /// C ABI：两整数相加（无副作用，最易讲清符号导出）
    #[no_mangle]
    pub extern "C" fn krs_lesson_add(a: i32, b: i32) -> i32 {
        a.saturating_add(b)
    }

    /// C ABI：把 NUL 结尾 ASCII 串长度返回（不读越界 —— 仍要求调用方传合法指针）
    #[no_mangle]
    pub extern "C" fn krs_mock_strlen(ptr: *const c_char) -> usize {
        if ptr.is_null() {
            return 0;
        }
        unsafe {
            let mut len = 0usize;
            let mut p = ptr;
            while *p != 0 {
                len += 1;
                p = p.add(1);
            }
            len
        }
    }
}

// Rust 侧声明「外部」符号（本课与子模块导出的符号在同一 binary，链接器会解析）
extern "C" {
    fn krs_lesson_add(a: i32, b: i32) -> i32;
    fn krs_mock_strlen(ptr: *const c_char) -> usize;
}

/// 直观版：直接 unsafe 调用（教学用；业务应再包一层）
fn call_add_naive(a: i32, b: i32) -> i32 {
    unsafe { krs_lesson_add(a, b) }
}

/// 进阶版：Safe 封装 —— 只对已知有效的 `CString` 暴露长度查询
struct SafeCString {
    inner: CString,
}

impl SafeCString {
    fn new(text: &str) -> Result<Self, std::ffi::NulError> {
        CString::new(text).map(|inner| Self { inner })
    }

    fn as_c_str(&self) -> &CStr {
        self.inner.as_c_str()
    }

    /// 通过 mock C ABI 数长度（指针来自 CString，保证 NUL 结尾）
    fn len_c_abi_mock(&self) -> usize {
        let ptr = self.inner.as_ptr();
        unsafe { krs_mock_strlen(ptr) }
    }

    /// 通过 libc `strlen`（系统 C 库；与本机已链接 libc 一致）
    fn len_c_abi_libc(&self) -> usize {
        let ptr = self.inner.as_ptr();
        unsafe { libc::strlen(ptr) }
    }
}

/// FFI 错误：把「不该暴露的 unsafe 前提」变成 `Result`
#[derive(Debug, PartialEq, Eq)]
enum FfiUserError {
    EmbeddedNul,
}

fn safe_add(a: i32, b: i32) -> i32 {
    // 纯 Rust 侧可先校验业务；此处 C 函数本身无额外契约
    call_add_naive(a, b)
}

fn safe_string_byte_len(s: &str) -> Result<usize, FfiUserError> {
    let wrapped = SafeCString::new(s).map_err(|_| FfiUserError::EmbeddedNul)?;
    Ok(wrapped.len_c_abi_mock())
}

fn run_tests() {
    assert_eq!(safe_add(40, 2), 42);
    assert_eq!(safe_add(i32::MAX, 1), i32::MAX); // saturating_add on C side

    let hi = SafeCString::new("hello").expect("CString");
    assert_eq!(hi.as_c_str().to_bytes(), b"hello");
    assert_eq!(hi.len_c_abi_mock(), 5);
    assert_eq!(hi.len_c_abi_libc(), 5);
    assert_eq!(safe_string_byte_len("ffi").unwrap(), 3);

    assert_eq!(
        safe_string_byte_len("has\0nul"),
        Err(FfiUserError::EmbeddedNul)
    );

    println!("✅ ffi_mental 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ FFI：C ABI 与 Safe CString ************\n");
    println!("krs_lesson_add(10, 32) = {}", safe_add(10, 32));

    if let Ok(n) = safe_string_byte_len("krs-project") {
        println!("C ABI 字符串字节长度（不含 NUL）: {n}");
    }

    run_tests();
}

fn main() {
    bootstrap();
}