# `18_unsafe_ffi` | Unsafe 与 FFI

- **目标**: 在大量 **safe Rust** 体感之后，建立 `unsafe` 边界心智、C ABI / `extern "C"` 调用习惯，以及对照 [Nomicon](https://doc.rust-lang.org/nomicon/) 的可测自检清单——**聚焦如何安全封装**，不练 exploit。
- **前置**: 就业主线至 `15_project_capstone`；精通线建议先完成 `16_production_rust`、`17_async_advanced`（本课注释按「可独立学习」写，但生产上应先会错误模型与 async）。
- **环境**: rustup 最新稳定版；验证：

```bash
./scripts/verify_modules.sh 18_unsafe_ffi
```

- **推荐资料（明文 URL，已核验）**:
- https://doc.rust-lang.org/nomicon/
- https://doc.rust-lang.org/reference/
- https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html

## 本课 ↔ 生产实践

| 本仓库（`cargo run` + `run_tests`） | 生产 / 团队协作 |
| --- | --- |
| `AlignedBuf` 集中 unsafe | 独立 `raw` 模块 / `-sys` crate，公开 safe API |
| 同 binary `#[no_mangle] extern "C"` | `libfoo.a` / `libfoo.so` + `build.rs` + `bindgen` |
| `SafeCString` 封装 | 所有跨 FFI 字符串必经拥有者类型 |
| `run_nomicon_checklist()` | Code Review + Miri / 模糊测试（另练） |

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_unsafe/src/main.rs](./00_unsafe/src/main.rs) | 裸指针、五类 unsafe 能力、`AlignedBuf` Safe 封装 | ✅ |
| `01_` | [01_ffi/src/main.rs](./01_ffi/src/main.rs) | `extern "C"`、 CString/CStr、mock C ABI + `libc::strlen` | ✅ |
| `02_` | [02_nomicon/src/main.rs](./02_nomicon/src/main.rs) | 对齐 / 别名 / 不变量可测清单 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 明文 URL |
| --- | --- |
| The Rustonomicon | https://doc.rust-lang.org/nomicon/ |
| Nomicon — FFI | https://doc.rust-lang.org/nomicon/ffi.html |
| Nomicon — 所有权 | https://doc.rust-lang.org/nomicon/ownership.html |
| Nomicon — 生命周期 | https://doc.rust-lang.org/nomicon/lifetimes.html |
| Rust Reference | https://doc.rust-lang.org/reference/ |
| Book — Unsafe Rust | https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html |
| Reference — external blocks | https://doc.rust-lang.org/reference/items/external-blocks.html |

**本目录总课数：3（已完成 3）**

## 学习顺序建议

1. `00_unsafe`：默认用 **safe 切片**；只有不变量写清时才把 `unsafe` 关进小函数。
2. `01_ffi`：分清 **Rust 所有权** 与 **C 谁 malloc/free**；`CString` 后再谈裸指针。
3. `02_nomicon`：写 unsafe PR 前跑一遍清单；再用 Miri 查悬垂与别名（可选）。

## 安全边界（本仓库硬性）

- 仅教边界封装与契约自检，不提供可被利用的内存破坏示例。
- 若对接真实 C 库：用 `bindgen` + 薄封装 crate，把 `unsafe` 调用次数压到最少。