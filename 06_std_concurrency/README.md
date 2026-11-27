# `06_std_concurrency` | 标准库并发

- **目标**: 用 **仅 std** 掌握 `thread`、`mpsc` channel、`Mutex`/`Arc`，并理解死锁等陷阱与可测规避；为 `07_async_tokio` 打基础（先同步线程模型，再学协作式 async）。
- **前置**: `05_cargo_modules`；强烈建议已完成 `01_ownership_borrowing`（`move` 闭包）、`03_error_collections`
- **环境**: rustup 最新稳定版；验证命令：

```bash
./scripts/verify_modules.sh 06_std_concurrency
```

- **推荐资料（明文 URL，已核验）**:
- https://doc.rust-lang.org/book/ch16-00-concurrency.html
- https://doc.rust-lang.org/book/ch16-01-threads.html
- https://doc.rust-lang.org/book/ch16-02-message-passing.html
- https://doc.rust-lang.org/book/ch16-03-shared-state.html
- https://doc.rust-lang.org/std/sync/struct.Mutex.html
- https://doc.rust-lang.org/std/sync/struct.Arc.html
- https://doc.rust-lang.org/std/sync/mpsc/
- https://www.youtube.com/@RustVideos

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_thread/src/main.rs](./00_thread/src/main.rs) | `thread::spawn`、`join`、`move` 闭包 | ✅ |
| `01_` | [01_channel/src/main.rs](./01_channel/src/main.rs) | `mpsc`、多生产者、所有权转移 | ✅ |
| `02_` | [02_mutex/src/main.rs](./02_mutex/src/main.rs) | `Mutex`、`Arc<Mutex<_>>` 共享计数 | ✅ |
| `03_` | [03_pitfalls/src/main.rs](./03_pitfalls/src/main.rs) | 死锁风险、`try_lock`、固定加锁顺序 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 明文 URL |
| --- | --- |
| 并发总览（The Book） | https://doc.rust-lang.org/book/ch16-00-concurrency.html |
| 线程 | https://doc.rust-lang.org/book/ch16-01-threads.html |
| 消息传递 | https://doc.rust-lang.org/book/ch16-02-message-passing.html |
| 共享状态 | https://doc.rust-lang.org/book/ch16-03-shared-state.html |
| `std::thread` | https://doc.rust-lang.org/std/thread/ |
| `mpsc` | https://doc.rust-lang.org/std/sync/mpsc/ |
| `Mutex` | https://doc.rust-lang.org/std/sync/struct.Mutex.html |
| `Arc` | https://doc.rust-lang.org/std/sync/struct.Arc.html |
| 视频频道 | https://www.youtube.com/@RustVideos |

**本目录总课数：4（已完成 4）**

## 学习顺序建议

1. 先跑 `00_thread`，理解「一闭包一线程」与 `join` 收结果。
2. 再学 `01_channel`：能传消息就不必抢着改同一块内存。
3. `02_mutex` 解决「必须共享可变状态」时的最小模式 `Arc<Mutex<T>>`。
4. `03_pitfalls` 用 **可结束的** 演示理解死锁；牢记：**固定锁顺序** 或 **`try_lock` 快速失败**，别让 CI/`verify` 永远挂起。