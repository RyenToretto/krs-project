/*
[主题]: Tokio runtime 与 spawn —— 谁在后台跑你的 async 任务
[难度]: Medium
[前置]: 07_async_tokio/00_async

一、主题说明
- **Runtime**：async 的「发动机」；负责在少量线程上切换很多任务（协作式调度）。
- `#[tokio::main]` 默认建好 **多线程** runtime（`rt-multi-thread`）。
- **`tokio::spawn`**：把一个 async 块丢到 runtime 里**并发**跑；返回 `JoinHandle`，像线程的 `join` 一样可以 `.await` 收结果。
- **Handle**：当前 runtime 的句柄；在不能写 `#[tokio::main]` 的库里，常 `Handle::current().block_on(...)`（本课点到为止）。

二、写法对比
1. 直观版：在 `main` async 里直接 `await` 一个个任务。
2. 进阶版：先 `spawn` 多个子任务，再 `join_all` 汇总（更符合「服务器里同时处理很多连接」的心智）。

三、真实用途
- 每个 TCP 连接一个 spawn 的 async 任务（简化模型）。
- 后台定时 flush 日志、_metrics 上报：spawn 后由 runtime 调度，别自己 `thread::spawn` 乱套。
资料: https://tokio.rs/tokio/tutorial
      https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html
      https://docs.rs/tokio/latest/tokio/task/fn.spawn.html
      https://rust-lang.github.io/async-book/
      https://www.youtube.com/@RustVideos
*/

use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// 子任务：sleep 后返回编号（模拟轻量后台工作）
async fn worker(id: u32, delay_ms: u64) -> u32 {
    sleep(Duration::from_millis(delay_ms)).await;
    id * 10
}

/// 直观版：在当前 async 调用栈里顺序 await
async fn sum_naive(ids: &[u32]) -> u32 {
    let mut total = 0;
    for &id in ids {
        total += worker(id, 8).await;
    }
    total
}

/// 进阶版：spawn 一批任务，再逐个 await JoinHandle
async fn sum_spawned(ids: &[u32]) -> u32 {
    let handles: Vec<JoinHandle<u32>> = ids
        .iter()
        .map(|&id| tokio::spawn(worker(id, 8)))
        .collect();

    let mut total = 0;
    for h in handles {
        total += h.await.expect("spawn 的任务 panic");
    }
    total
}

async fn run_tests() {
    let ids = [1u32, 2, 3, 4];
    assert_eq!(sum_naive(&ids).await, 100); // 10+20+30+40
    assert_eq!(sum_spawned(&ids).await, 100);

    let h = tokio::spawn(async { 7 });
    assert_eq!(h.await.unwrap(), 7);

    // 子任务 panic 时 `JoinHandle::await` 会得到 `Err(JoinError)`（本课不在 verify 里真 panic，避免干扰日志）
    let nested = tokio::spawn(async { tokio::spawn(async { 3 + 4 }).await.unwrap() });
    assert_eq!(nested.await.unwrap(), 7);

    println!("✅ Tokio runtime / spawn 所有测试用例通过！");
}

async fn bootstrap() {
    println!("\n\n************ L01 Tokio runtime / spawn ************\n");

    let ids = vec![1, 2, 3];
    println!("顺序 await workers: {}", sum_naive(&ids).await);
    println!("spawn 后汇总: {}", sum_spawned(&ids).await);

    println!(
        "当前在 Tokio runtime 里: is_tokio = {}",
        tokio::runtime::Handle::try_current().is_ok()
    );
}

#[tokio::main]
async fn main() {
    bootstrap().await;
    run_tests().await;
}