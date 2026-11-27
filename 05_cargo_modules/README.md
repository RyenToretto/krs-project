# `05_cargo_modules` | Cargo 与模块系统

- **目标**: 掌握 `mod` / 可见性、`Cargo.toml` 与 **features**、**workspace** 多 crate 协作心智；为并发、Web 工程化打基础。
- **前置**: `04_traits_generics`（建议）；`03_error_collections` 的 `Result` 已会即可开本阶段。
- **环境**: rustup 最新稳定版；验证命令：

```bash
./scripts/verify_modules.sh 05_cargo_modules
```

- **推荐资料（明文 URL，已核验）**:
- https://doc.rust-lang.org/cargo/
- https://doc.rust-lang.org/cargo/guide/
- https://doc.rust-lang.org/cargo/reference/workspaces.html
- https://doc.rust-lang.org/cargo/reference/features.html
- https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
- https://docs.rs/
- https://www.youtube.com/@RustVideos

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_modules/src/main.rs](./00_modules/src/main.rs) | `mod`、`pub` / `pub(crate)` / `pub(super)`、`pub use` | ✅ |
| `01_` | [01_cargo/src/main.rs](./01_cargo/src/main.rs) | `Cargo.toml`、`#[cfg(feature)]`、`cfg!` | ✅ |
| `02_` | [02_workspace/src/main.rs](./02_workspace/src/main.rs) | workspace 成员、`path` 依赖、统一 lock（字符串心智 + 可测解析） | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 视频 / 教程 / 书籍（明文 URL） |
| --- | --- |
| 包、Crate 与模块总览 | https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html |
| 定义模块与隐私 | https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html |
| Cargo 手册 | https://doc.rust-lang.org/cargo/ |
| Cargo 指南 | https://doc.rust-lang.org/cargo/guide/ |
| Workspaces | https://doc.rust-lang.org/cargo/reference/workspaces.html |
| Features | https://doc.rust-lang.org/cargo/reference/features.html |
| crates.io 文档入口 | https://docs.rs/ |
| The Book 总览 | https://doc.rust-lang.org/book/ |
| 视频 | https://www.youtube.com/@RustVideos |

**本目录总课数：3（已完成 3）**