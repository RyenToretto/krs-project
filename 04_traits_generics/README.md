# `04_traits_generics` | Trait 与泛型

- **目标**: 掌握泛型函数与结构体、trait 定义与 `impl`、默认方法，以及 `impl Trait` / `dyn Trait` 的静态 vs 动态分发直觉；为后续错误类型抽象、trait bound、生产里可测试设计打基础。
- **前置**: `03_error_collections`（`Result`、`Vec`）；`02_structs_enums`
- **环境**: rustup 最新稳定版；验证命令：

```bash
./scripts/verify_modules.sh 04_traits_generics
```

- **推荐资料（明文 URL，已核验）**:
- https://doc.rust-lang.org/book/ch10-00-generics.html
- https://doc.rust-lang.org/book/ch10-01-syntax.html
- https://doc.rust-lang.org/book/ch10-02-traits.html
- https://rust-lang.github.io/api-guidelines/
- https://doc.rust-lang.org/book/
- https://doc.rust-lang.org/rust-by-example/trait.html
- https://doc.rust-lang.org/rust-by-example/generics.html
- https://www.youtube.com/@RustVideos

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_generics/src/main.rs](./00_generics/src/main.rs) | 泛型函数、`BoxValue<T>`、`Pair<T,U>`、`T: PartialOrd` | ✅ |
| `01_` | [01_traits/src/main.rs](./01_traits/src/main.rs) | trait / `impl`、默认方法、`impl Summary` 约束 | ✅ |
| `02_` | [02_dyn/src/main.rs](./02_dyn/src/main.rs) | 静态分发 vs `dyn Trait`、`Box<dyn Trait>` | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 视频 / 教程 / 书籍（明文 URL） |
| --- | --- |
| 泛型总览 | https://doc.rust-lang.org/book/ch10-00-generics.html |
| 泛型语法 | https://doc.rust-lang.org/book/ch10-01-syntax.html |
| Trait | https://doc.rust-lang.org/book/ch10-02-traits.html |
| Trait 对象（深入） | https://doc.rust-lang.org/book/ch17-02-trait-objects.html |
| RBE 泛型 | https://doc.rust-lang.org/rust-by-example/generics.html |
| RBE Trait | https://doc.rust-lang.org/rust-by-example/trait.html |
| API 设计指南 | https://rust-lang.github.io/api-guidelines/ |
| The Book 总览 | https://doc.rust-lang.org/book/ |
| 视频 | https://www.youtube.com/@RustVideos |

**本目录总课数：3（已完成 3）**