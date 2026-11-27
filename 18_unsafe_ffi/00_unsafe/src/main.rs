/*
[主题]: unsafe 边界 —— 原始指针、不变量与 Safe 封装
[难度]: Medium
[前置]: `01_ownership_borrowing`、`04_traits_generics`（生命周期）、`15_project_capstone` 建议

一、主题说明
- **safe Rust** 在编译期保证内存安全；`unsafe` 不是「绕过规则耍帅」，而是把五类能力包进**极小**的块里，并由你**文档化 + 测试**不变量。
- 五类能力（Book 20.1）：解引用裸指针、调用 unsafe 函数、访问可变静态、实现 unsafe trait、访问 union 字段。
- **裸指针** `*const T` / `*mut T` 可随时创建，但解引用必须在 `unsafe` 中，且你要自己保证：对齐、合法地址、无悬垂、别名规则。
- 生产实践：**几乎全部业务代码保持 safe**；`unsafe` 集中在 `mod raw { ... }` 或单独 crate，对外只暴露 Safe API。

二、写法对比
1. 直观版: `chunks_exact(4)` + `try_into` 读 `u32`，全程 safe，适合 99% 协议解析。
2. 进阶版: `AlignedBuf` 把「指针 + 长度 + 独占所有权」封装起来，内部用 `slice::from_raw_parts` / `ptr::read_unaligned`，外部只调 `read_u32_le(offset)`。

三、真实用途
- `Vec`/`String` 内部、`std::sync` 原子、序列化零拷贝、嵌入式寄存器映射、与 C 缓冲区对接前的 Rust 侧视图。
- 读内核/驱动前先在用户态写好 **Safe wrapper + Miri/测试**（本课用 `run_tests` 表驱动）。
资料: https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html
      https://doc.rust-lang.org/nomicon/
      https://doc.rust-lang.org/reference/
*/

use std::convert::TryInto;
use std::ptr;

/// 直观版：用标准库切片 API 读 little-endian `u32`（推荐默认写法）
fn read_u32_le_safe(data: &[u8], offset: usize) -> Result<u32, &'static str> {
    let chunk = data
        .get(offset..offset + 4)
        .ok_or("偏移越界")?;
    let bytes: [u8; 4] = chunk.try_into().map_err(|_| "长度不是 4")?;
    Ok(u32::from_le_bytes(bytes))
}

/// 持有独占缓冲的不变量：`ptr` 来自 `Vec`，`len` 与 Vec 一致，Drop 时交还 Vec。
struct AlignedBuf {
    vec: Vec<u8>,
}

impl AlignedBuf {
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            vec: bytes.to_vec(),
        }
    }

    /// 对外安全 API：先检查边界与对齐意图，再集中一处 unsafe 读内存
    fn read_u32_le(&self, offset: usize) -> Result<u32, &'static str> {
        if offset + 4 > self.vec.len() {
            return Err("偏移越界");
        }
        read_u32_le_unsafe_inner(self.vec.as_ptr(), offset)
    }
}

/// 进阶版：unsafe 被关在函数里；调用方须提供「ptr 在 offset..offset+4 可读」的不变量
fn read_u32_le_unsafe_inner(base: *const u8, offset: usize) -> Result<u32, &'static str> {
    if base.is_null() {
        return Err("空指针");
    }
    // SAFETY: 调用方保证 base 指向有效分配，且 offset..offset+4 在分配范围内。
    // 此处用 read_unaligned 演示「未对齐地址也可能合法」；生产仍优先 safe 切片。
    unsafe {
        let p = base.add(offset);
        let v = ptr::read_unaligned(p as *const u32);
        Ok(v.to_le())
    }
}

/// 演示：裸指针可自由创建（不等价于合法解引用）
fn demo_raw_ptr_metadata(data: &[u8]) -> (*const u8, usize) {
    (data.as_ptr(), data.len())
}

/// `SafeView`：只暴露共享引用，内部记录指针仅用于调试打印，不解引用
struct SafeView<'a> {
    inner: &'a [u8],
}

impl<'a> SafeView<'a> {
    fn new(slice: &'a [u8]) -> Self {
        Self { inner: slice }
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn as_safe_slice(&self) -> &[u8] {
        self.inner
    }
}

#[derive(Debug, Clone, Copy)]
struct ReadCase {
    name: &'static str,
    data: &'static [u8],
    offset: usize,
    want: Result<u32, &'static str>,
}

const READ_CASES: &[ReadCase] = &[
    ReadCase {
        name: "偏移 0 读 hello 前 4 字节",
        data: b"hello",
        offset: 0,
        want: Ok(u32::from_le_bytes(*b"hell")),
    },
    ReadCase {
        name: "越界",
        data: b"ab",
        offset: 0,
        want: Err("偏移越界"),
    },
];

fn run_table(cases: &[ReadCase], label: &str) {
    for c in cases {
        let got_safe = read_u32_le_safe(c.data, c.offset);
        assert_eq!(got_safe, c.want, "[{}][safe] {}", label, c.name);

        let buf = AlignedBuf::from_bytes(c.data);
        let got_unsafe = buf.read_u32_le(c.offset);
        assert_eq!(got_unsafe, c.want, "[{}][AlignedBuf] {}", label, c.name);
    }
}

fn run_tests() {
    run_table(READ_CASES, "u32_le");

    let sample = [1u8, 2, 3, 4, 5];
    let (p, len) = demo_raw_ptr_metadata(&sample);
    assert!(!p.is_null());
    assert_eq!(len, 5);

    let view = SafeView::new(&sample);
    assert_eq!(view.len(), 5);
    assert_eq!(view.as_safe_slice(), &sample[..]);

    // 不变量：AlignedBuf 与 safe 路径在合法输入下结果一致
    let buf = AlignedBuf::from_bytes(&sample);
    assert_eq!(read_u32_le_safe(&sample, 0).unwrap(), buf.read_u32_le(0).unwrap());

    println!("✅ unsafe_basics 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ unsafe 边界：指针与 Safe 封装 ************\n");
    println!("裸指针 ({:p}, len={}) —— 创建是 safe 的，解引用才需要 unsafe + 不变量", {
        let (p, _) = demo_raw_ptr_metadata(b"demo");
        p
    }, 4);

    let buf = AlignedBuf::from_bytes(&[0xef, 0xbe, 0xad, 0xde, 0]);
    if let Ok(v) = buf.read_u32_le(0) {
        println!("AlignedBuf 读出 little-endian u32: 0x{v:08x}");
    }

    run_tests();
}

fn main() {
    bootstrap();
}