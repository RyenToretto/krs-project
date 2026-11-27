# `16_production_rust` | 生产级 Rust 工程（精通线）

- **目标**: 在就业主线毕业项目之上，补齐**可上线**的工程能力：分层错误、配置合并与校验、重试/熔断、Clippy + API Guidelines 自检习惯。
- **前置**: `15_project_capstone`（分层 API + 统一错误枚举）
- **环境**: rustup 最新稳定版；验证：`./scripts/verify_modules.sh 16_production_rust`
- **难度**: 每课含**陷阱清单**、**生产组件对照表**、`run_tests()` 断言。

## 推荐资料（明文 URL，已核验）

| 资料 | URL |
| --- | --- |
| thiserror | https://docs.rs/thiserror/latest/thiserror/ |
| anyhow | https://docs.rs/anyhow/latest/anyhow/ |
| API Guidelines | https://rust-lang.github.io/api-guidelines/ |
| Clippy 书 | https://doc.rust-lang.org/clippy/ |
| Clippy lint 列表 | https://rust-lang.github.io/rust-clippy/master/ |
| Tower Retry | https://docs.rs/tower/latest/tower/retry/struct.Retry.html |
| config crate | https://docs.rs/config/latest/config/ |
| figment | https://docs.rs/figment/latest/figment/ |
| SRE 级联故障 | https://sre.google/sre-book/addressing-cascading-failures/ |

## 章节进度

| 编号 | 目录 / 包 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | `00_errors` / `l00_thiserror_anyhow` | 库 thiserror、bin anyhow、错误链与 HTTP 映射陷阱 | ✅ |
| `01_` | `01_config` / `l00_config_mental` | 默认→文件→env 分层、校验、Redacted 脱敏 | ✅ |
| `02_` | `02_resilience` / `l00_resilience` | 指数退避、熔断态机、幂等重试陷阱 | ✅ |
| `03_` | `03_clippy` / `l00_clippy_api_guidelines` | 清单式 Guideline 扫描、Clippy 心智 | ✅ |

## 本阶段知识点资料包

| 知识点 | 说明 | 明文 URL |
| --- | --- | --- |
| 分层错误 | 库枚举 + `#[source]` / `thiserror`；`main` 用 `anyhow::Context` | https://docs.rs/thiserror/latest/thiserror/ ；https://docs.rs/anyhow/latest/anyhow/ |
| 配置 | merge 顺序、类型解析、启动校验、秘密脱敏 | https://docs.rs/config/latest/config/ ；https://docs.rs/figment/latest/figment/ |
| 弹性 | 瞬态 vs 致命、退避抖动、熔断 Closed/Open/HalfOpen | https://docs.rs/tower/latest/tower/retry/struct.Retry.html ；https://sre.google/sre-book/addressing-cascading-failures/ |
| 代码质量 | `cargo clippy -D warnings` + Guidelines C-* 自查 | https://doc.rust-lang.org/clippy/ ；https://rust-lang.github.io/api-guidelines/ |

## 与就业线的关系

- `15_project_capstone` 里已有 `AppError` 与 HTTP status 映射；本阶段 `00_errors` 把它推进到 **crate 边界规范**（库不向公开 API 泄漏 `anyhow`）。
- `14_observability` 的 tracing 与本阶段 **context / 脱敏日志** 结合，才能在事故里同时看到调用链与配置快照（不含秘密）。

**本目录总课数：4（已完成 4）**