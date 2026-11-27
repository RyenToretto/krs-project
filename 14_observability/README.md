# `14_observability` | 可观测性

- **目标**: 用纯 Rust 迷你实现建立 **tracing（日志事件 + Span）**、**指标（Counter / Histogram）**、**OpenTelemetry（Trace 传播与三信号）** 心智；运行秒级结束、无第三方可观测 crate。
- **前置**: `07_async_tokio`（请求链路）；`08_web_axum`（HTTP 与中间件）；`12_grpc_tonic/00_split`（跨服务调用，便于理解 traceparent）。
- **环境**: rustup 最新稳定版；验证：

```bash
./scripts/verify_modules.sh 14_observability
```

## 推荐资料（明文 URL）

| 来源 | 明文 URL |
| --- | --- |
| tracing API | https://docs.rs/tracing/latest/tracing/ |
| tracing 仓库与书 | https://github.com/tokio-rs/tracing |
| OpenTelemetry Rust | https://opentelemetry.io/docs/languages/rust/ |

## 章节进度

| 编号 | 目录 / 包 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_tracing/` | `l00_tracing_mental` | Event、Span 栈、Subscriber 与 Level 过滤 | ✅ |
| `01_metrics/` | `l00_metrics_mental` | Counter、Histogram 桶、Registry、Prometheus 风格导出 | ✅ |
| `02_otel/` | `l00_otel_mental` | TraceId/SpanId、W3C `traceparent`、内存 Exporter 与三信号管线 | ✅ |

## 本阶段知识点资料包

| 知识点 | 说明 | 明文 URL |
| --- | --- | --- |
| 结构化日志 | `tracing::Level`、`target`、fields | https://docs.rs/tracing/latest/tracing/ |
| Span 与订阅者 | `Subscriber`、`Layer`、`tracing-subscriber` | https://github.com/tokio-rs/tracing |
| 指标 | Prometheus 文本、直方图 `_bucket/_sum/_count` | https://opentelemetry.io/docs/languages/rust/ |
| 分布式追踪 | W3C Trace Context、`traceparent` | https://opentelemetry.io/docs/languages/rust/ |
| 与 Axum 结合 | 中间件注入 span / 请求指标（生产用真实 crate） | https://docs.rs/tracing/latest/tracing/ |

## 单课运行

```bash
cargo run --manifest-path 14_observability/00_tracing/Cargo.toml
cargo run --manifest-path 14_observability/01_metrics/Cargo.toml
cargo run --manifest-path 14_observability/02_otel/Cargo.toml
```

**本目录总课数：3（已全部生成）**