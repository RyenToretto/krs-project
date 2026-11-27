# `07_async_tokio` | 异步与 Tokio

- **目标**: 从零理解 `async`/`await`、Tokio **runtime**、`spawn`、**`select!` / `join!`**，以及 **在 async 里阻塞** 的常见陷阱与 `spawn_blocking` 正路；为 `08_web_axum` 打地基。
- **前置**: `06_std_concurrency`（尤其 `00_thread`）；不要求事先会 Tokio。
- **环境**: rustup 最新稳定版；验证命令：

```bash
./scripts/verify_modules.sh 07_async_tokio
```

- **推荐资料（明文 URL，已核验）**:
- https://doc.rust-lang.org/stable/book/ch17-00-async-await.html
- https://rust-lang.github.io/async-book/
- https://tokio.rs/tokio/tutorial
- https://docs.rs/tokio/latest/tokio/
- https://tokio.rs/
- https://www.youtube.com/@RustVideos

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_async/src/main.rs](./00_async/src/main.rs) | `async fn`、`.await`、`#[tokio::main]`、`join!` 并发等待 | ✅ |
| `01_` | [01_runtime/src/main.rs](./01_runtime/src/main.rs) | runtime、`tokio::spawn`、`JoinHandle` | ✅ |
| `02_` | [02_select/src/main.rs](./02_select/src/main.rs) | `select!` 竞速、`join!` / `try_join!` | ✅ |
| `03_` | [03_pitfalls/src/main.rs](./03_pitfalls/src/main.rs) | 阻塞拖垮 worker、`spawn_blocking`、合作式 `sleep` | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 明文 URL |
| --- | --- |
| The Book 异步章 | https://doc.rust-lang.org/stable/book/ch17-00-async-await.html |
| Async Book | https://rust-lang.github.io/async-book/ |
| Tokio 教程 | https://tokio.rs/tokio/tutorial |
| Tokio 文档 | https://docs.rs/tokio/latest/tokio/ |
| Tokio 官网 | https://tokio.rs/ |
| `select!` | https://docs.rs/tokio/latest/tokio/macro.select.html |
| `join!` | https://docs.rs/tokio/latest/tokio/macro.join.html |
| `spawn` | https://docs.rs/tokio/latest/tokio/task/fn.spawn.html |
| `spawn_blocking` | https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html |
| 视频频道 | https://www.youtube.com/@RustVideos |

**本目录总课数：4（已完成 4）**

## 学习顺序建议

1. `00_async`：先建立「async 不会立刻跑完、要靠 runtime + await」的直觉；对比顺序 await 与 `join!`。
2. `01_runtime`：理解谁在执行任务；练习 `spawn` 与收 `JoinHandle`。
3. `02_select`：需要「谁先完成」用 `select!`；需要「全部完成」用 `join!` / `try_join!`。
4. `03_pitfalls`：牢记 **async 里别 `thread::sleep` 挡路**；真阻塞用 `spawn_blocking` 或换成 async API。

## 离线说明

各课用 `tokio::time::sleep` 与毫秒级 `Instant` 自测，**不访问外网**；依赖仅 `tokio`（`rt-multi-thread`, `macros`, `sync`, `time`）。