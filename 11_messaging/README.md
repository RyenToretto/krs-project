# `11_messaging` | 消息与异步解耦

- **目标**: 用内存假 Broker 掌握「发布后立即返回 → 后台消费」、**至少一次投递 + 幂等**、**Topic / Consumer Group / ACK** 心智；全程 **不连 NATS/Kafka**，离线可测。
- **前置**: `10_caching_redis`（建议）；`08_web_axum`（Handler 分层）；`07_async_tokio`（真异步跑 Consumer 时再看）
- **环境**: rustup 稳定版，`edition = 2021`；验证：`./scripts/verify_modules.sh 11_messaging`
- **推荐资料（明文 URL，已核验）**:
  - https://nats.io/docs/
  - https://docs.rs/rdkafka/latest/rdkafka/
  - https://www.youtube.com/@RustVideos

## 章节进度

| 编号 | 目录 | 包名 | 核心点 | 状态 |
| --- | --- | --- | --- | --- |
| `00_` | `00_async/` | `l00_async_decouple` | `EventPublisher` + `InMemoryBroker`，API 只 publish，Worker drain 扇出 Handler | ✅ |
| `01_` | `01_idempotent/` | `l00_at_least_once_idempotent` | 未 ACK 重投、`ProcessedStore` 按 `message_id` 幂等扣款 | ✅ |
| `02_` | `02_broker/` | `l00_broker_mental` | `MiniBroker`：topic 日志、consumer group 轮询、ACK/NACK | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 说明 | 明文 URL |
| --- | --- | --- |
| NATS 文档 | subject、queue group、JetStream 持久化与 ACK | https://nats.io/docs/ |
| rdkafka | Kafka 生产/消费 API、consumer group、offset | https://docs.rs/rdkafka/latest/rdkafka/ |
| Rust 视频 | 补充听/看 | https://www.youtube.com/@RustVideos |

## 与真实中间件的对照

| 本课概念 | 生产里大致是什么 |
| --- | --- |
| `InMemoryBroker::publish` | `async_nats::Client::publish` / `FutureProducer::send` |
| `poll` + `inflight` + `ack` | JetStream `ack` / Kafka manual commit offset |
| `message_id` 幂等表 | DB `UNIQUE(message_id)`、Redis `SETNX`、业务状态机 |
| `consumer group` 轮询 | Kafka partition 分配 / NATS queue subscriber |
| 多 Handler 扇出 | 多 consumer group 各读一份，或单独 fan-out topic |

## 运行单课

```bash
cargo run --manifest-path 11_messaging/00_async/Cargo.toml
cargo run --manifest-path 11_messaging/01_idempotent/Cargo.toml
cargo run --manifest-path 11_messaging/02_broker/Cargo.toml
```

**本目录总课数：3（已完成 3）**