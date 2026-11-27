# `01_ownership_borrowing` | 所有权与借用

- **目标**: 掌握所有权 move、不可变/可变借用、切片、生命周期标注的入门直觉；能读懂编译器常见的 borrow 报错。
- **前置**: `00_rust_basics`
- **环境**: rustup 最新稳定版；`./scripts/verify_modules.sh 01_ownership_borrowing`
- **推荐资料（明文 URL，已核验）**:
- https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html
- https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html
- https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html
- https://doc.rust-lang.org/book/ch04-03-slices.html
- https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html
- https://doc.rust-lang.org/book/
- https://doc.rust-lang.org/rust-by-example/
- https://doc.rust-lang.org/rust-by-example/scope/move.html
- https://doc.rust-lang.org/rust-by-example/scope/borrow.html
- https://www.youtube.com/@RustVideos
- https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_move/src/main.rs](./00_move/src/main.rs) | 所有权、move、Copy、`String` | ✅ |
| `01_` | [01_borrow/src/main.rs](./01_borrow/src/main.rs) | `&T`、`&mut T`、借用规则 | ✅ |
| `02_` | [02_slice/src/main.rs](./02_slice/src/main.rs) | `&[T]`、`&str`、子区间视图 | ✅ |
| `03_` | [03_lifetime_intro/src/main.rs](./03_lifetime_intro/src/main.rs) | 简单 `'a`、返回引用直觉 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 视频 / 教程 / 书籍（明文 URL） |
| --- | --- |
| 所有权总览 | https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html |
| 什么是所有权 | https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html |
| 引用与借用 | https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html |
| 切片 | https://doc.rust-lang.org/book/ch04-03-slices.html |
| 生命周期语法 | https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html |
| The Book | https://doc.rust-lang.org/book/ |
| Rust By Example | https://doc.rust-lang.org/rust-by-example/ |
| RBE move | https://doc.rust-lang.org/rust-by-example/scope/move.html |
| RBE borrow | https://doc.rust-lang.org/rust-by-example/scope/borrow.html |
| 视频 | https://www.youtube.com/@RustVideos |
| 书 | https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590 |

**本目录总课数：4（已完成 4）**