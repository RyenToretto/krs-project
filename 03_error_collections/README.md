# `03_error_collections` | 错误处理与集合

- **目标**: 掌握 `Result` / `?`、`Vec`、`HashMap`、`String` 进阶（容量与 UTF-8）；为后续 trait、错误库（`thiserror` 等）打基础。
- **前置**: `02_structs_enums`（建议已学 Option）；`00_rust_basics` 中的字符串入门
- **环境**: rustup 最新稳定版；验证命令：

```bash
./scripts/verify_modules.sh 03_error_collections
```

- **推荐资料（明文 URL，已核验）**:
- https://doc.rust-lang.org/book/
- https://doc.rust-lang.org/book/ch09-00-error-handling.html
- https://doc.rust-lang.org/book/ch08-00-common-collections.html
- https://doc.rust-lang.org/std/
- https://doc.rust-lang.org/rust-by-example/
- https://www.youtube.com/@RustVideos

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_result/src/main.rs](./00_result/src/main.rs) | `Result`、`Ok`/`Err`、`?`、错误冒泡 | ✅ |
| `01_` | [01_vec_map/src/main.rs](./01_vec_map/src/main.rs) | `Vec`、`HashMap`、`entry` | ✅ |
| `02_` | [02_string/src/main.rs](./02_string/src/main.rs) | `String` / `&str`、容量、`from_utf8` | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 视频 / 教程 / 书籍（明文 URL） |
| --- | --- |
| 错误处理总览 | https://doc.rust-lang.org/book/ch09-00-error-handling.html |
| `Result` | https://doc.rust-lang.org/std/result/ |
| 常用集合 | https://doc.rust-lang.org/book/ch08-00-common-collections.html |
| `Vec` | https://doc.rust-lang.org/std/vec/struct.Vec.html |
| `HashMap` | https://doc.rust-lang.org/std/collections/struct.HashMap.html |
| 字符串 | https://doc.rust-lang.org/book/ch08-02-strings.html ；https://doc.rust-lang.org/std/string/struct.String.html |
| The Book 总览 | https://doc.rust-lang.org/book/ |
| Rust By Example | https://doc.rust-lang.org/rust-by-example/ |
| 视频 | https://www.youtube.com/@RustVideos |

**本目录总课数：3（已完成 3）**