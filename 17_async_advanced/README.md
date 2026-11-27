# `17_async_advanced` | 异步深挖（精通线）

- **目标**：在已会 `async/await` 与 Tokio 日常用法的前提下，把 **Future 状态机**、**Pin 保证**、**协作式取消**与 **生产陷阱** 讲到能读源码、能 Code Review。
- **前置**：`07_async_tokio/`（尤其 `00_async`、`02_select`）、`16_production_rust/`（建议先过生产工程心智）。
- **环境**：rustup 最新稳定版；本阶段课均依赖 **Tokio 1**（离线、毫秒级 sleep，verify 不 hang）。
- **验证**：

```bash
./scripts/verify_modules.sh 17_async_advanced
```

## 推荐资料（明文 URL，已核验）

- https://rust-lang.github.io/async-book/
- https://doc.rust-lang.org/std/pin/struct.Pin.html
- https://docs.rs/tokio/latest/tokio/
- https://doc.rust-lang.org/stable/book/ch17-00-async-await.html
- https://doc.rust-lang.org/std/future/trait.Future.html

## 章节进度

| 编号 | 目录 / 包名 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | `00_future` / `l00_future_intuition` | `Future::poll`、`Pending`/`Ready`、Waker、async 状态机 | ✅ |
| `01_` | `01_pin` / `l00_pin_mental` | `Pin`、自引用、`Unpin`、`poll(Pin<&mut Self>)` | ✅ |
| `02_` | `02_cancel` / `l00_cancellation` | `abort`、`select!` + `oneshot`、超时取消 | ✅ |
| `03_` | `03_traps` / `l00_async_traps` | 持锁跨 await、`tokio::sync::Mutex`、`select` 善后 | ✅ |

## 本阶段知识点资料包

| 知识点 | 说明 | 明文 URL |
| --- | --- | --- |
| Async Book | 异步语义与执行模型 | https://rust-lang.github.io/async-book/ |
| `Pin` 官方文档 | 钉扎与自引用 | https://doc.rust-lang.org/std/pin/struct.Pin.html |
| Tokio 文档 | 运行时、任务、同步原语 | https://docs.rs/tokio/latest/tokio/ |
| The Book 第 17 章 | `async/await` 语法入门对照 | https://doc.rust-lang.org/stable/book/ch17-00-async-await.html |
| `Future` trait | `poll` 契约 | https://doc.rust-lang.org/std/future/trait.Future.html |
| `JoinHandle::abort` | 任务取消入口 | https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html |
| `tokio::select!` | 多路竞速与取消模式 | https://docs.rs/tokio/latest/tokio/macro.select.html |
| `tokio::sync::Mutex` | 异步互斥锁 | https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html |

**本目录总课数：4（已完成 4）**