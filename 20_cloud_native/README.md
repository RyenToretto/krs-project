# `20_cloud_native` | 云原生（精通线）

- **目标**: 在**无需真 Docker/K8s**的前提下，用可运行迷你实现建立容器心智（环境 / 只读根 / 信号）、**liveness vs readiness** 健康检查分离、云原生**配置与密钥**（env + 挂载 + 日志脱敏）。
- **前置**: `16_production_rust`（配置分层与 Redacted）、`14_observability`（结构化日志）、`08_web_axum`（路由）
- **环境**: rustup 最新稳定版；验证：`./scripts/verify_modules.sh 20_cloud_native`
- **说明**: 容器运行时与镜像构建的官方文档可在具备网络时自行查阅；本仓库课内用模拟器离线可测，可观测场景对照 **tracing**。

## 推荐资料（明文 URL，已核验）

| 资料 | URL |
| --- | --- |
| The Rust Book | https://doc.rust-lang.org/book/ |
| Rust Embedded Book（资源约束 / 部署心智） | https://doc.rust-lang.org/embedded-book/ |
| Rust 嵌入概述 | https://www.rust-lang.org/what/embedded |
| tracing（结构化日志与字段脱敏） | https://docs.rs/tracing/latest/tracing/ |

## 章节进度

| 编号 | 目录 / 包 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | `00_container` / `l00_container_mental` | 编排注入 env、只读根与可写挂载、SIGTERM drain | ✅ |
| `01_` | `01_health` / `l00_health_checks` | `/livez` / `/readyz` 分离、探针注册表、重启 vs 摘流陷阱 | ✅ |
| `02_` | `02_config` / `l00_config_secrets` | Secret 卷 + env 覆盖、`SecretString`、结构化日志脱敏 | ✅ |

## 本阶段知识点资料包

| 知识点 | 说明 | 明文 URL |
| --- | --- | --- |
| 容器约束 | 非 VM；只读镜像层 + 卷；宽限期停机 | https://doc.rust-lang.org/book/ ；本课 `MiniRuntime` 模拟 |
| 健康检查 | Liveness 重启、Readiness 摘流量；勿混用慢探针 | https://docs.rs/tracing/latest/tracing/ （记录 probe 失败） |
| 配置与密钥 | ConfigMap vs Secret；勿日志明文；Debug 不脱敏陷阱 | https://doc.rust-lang.org/book/ ；对照 `16_production_rust/01_config` |
| 可观测 | 字段级脱敏对照 `tracing::info!(x = %redacted)` | https://docs.rs/tracing/latest/tracing/ |
| 嵌入式部署 | 资源受限环境下的构建与静态链接心智 | https://doc.rust-lang.org/embedded-book/ ；https://www.rust-lang.org/what/embedded |

## 与前后阶段的关系

- `16_production_rust/01_config` 讲 **默认 → 文件 → env** 分层；本阶段 `02_config` 强调 **Secret 面** 与编排挂载路径。
- `15_project_capstone` 的 `/health` 在本阶段拆成 **live / ready**，避免生产误重启。
- 下一 stage `21_architecture` 的 SRE 清单会收束可用性、变更与事故流程。

**本目录总课数：3（已完成 3）**