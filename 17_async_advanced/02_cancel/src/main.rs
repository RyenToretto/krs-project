/*
[主题]: 异步取消 ——  cooperatively 停下长跑任务
[难度]: Hard
[前置]: 07_async_tokio/02_select、17_async_advanced/00_future

一、主题说明
- **Rust 没有「线程 kill」式的粗暴取消**：async 任务一般在 `.await` 边界 **协作式** 停下。
- **常见手段**：
  1. **`JoinHandle::abort()`**：请求 Tokio 尽快终止任务（在 await 点生效；正在 poll 的同步代码不会立刻停）。
  2. **`select!`**：一路是业务，一路是关闭信号 / 超时 → 先完成者赢。
  3. **`oneshot` / `watch` 通道**：发「请退出」；任务循环里检查或 `select!`。
  4. **Drop 守卫**：结构体 drop 时关通道，子任务感知退出（本课简化演示）。
- **陷阱**：被取消的任务若持有锁 / 事务，清理要靠 `Drop` 或显式 `defer` 模式；Axum 断开连接 ≠ 自动取消你的 spawn。

二、写法对比
1. 直观版 `abort_long_task`：`abort()` 后长时间循环任务应无法完成原目标。
2. 进阶版 `work_with_cancel_token`：`select!` 在「假 I/O」与「取消信号」间竞速。
3. `timeout_via_select`：用 `sleep` 模拟超时取消（毫秒级，离线）。

三、真实用途
- 客户端断连后停止昂贵计算；shutdown 时等所有 task 退出。
- Kubernetes preStop + graceful cancel。
- 与 `tokio_util::sync::CancellationToken` 同类心智（本课不引新 crate，手写等价物）。

资料: https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html
      https://docs.rs/tokio/latest/tokio/macro.select.html
      https://rust-lang.github.io/async-book/
      https://docs.rs/tokio/latest/tokio/
*/

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::sleep;

// ─────────────────────────────────────────────
// 直观版：abort —— 请求运行时取消 JoinHandle
// ─────────────────────────────────────────────

async fn counting_while_alive(counter: Arc<AtomicU32>, stop_at: u32) -> Result<u32, &'static str> {
    loop {
        let n = counter.fetch_add(1, Ordering::SeqCst);
        if n >= stop_at {
            return Ok(n);
        }
        sleep(Duration::from_millis(1)).await;
    }
}

async fn abort_stops_progress() -> bool {
    let counter = Arc::new(AtomicU32::new(0));
    let c = Arc::clone(&counter);
    let handle = tokio::spawn(async move {
        let _ = counting_while_alive(c, 10_000).await;
    });

    sleep(Duration::from_millis(8)).await;
    handle.abort();
    let _ = handle.await; // JoinError: cancelled

    // 若未 abort，8ms 内 1ms 步进大约会到 ~8；abort 后不应接近 10000
    counter.load(Ordering::SeqCst) < 200
}

// ─────────────────────────────────────────────
// 进阶版：select! + oneshot 取消令牌
// ─────────────────────────────────────────────

async fn fake_io_work(ms: u64) -> &'static str {
    sleep(Duration::from_millis(ms)).await;
    "done"
}

#[derive(Debug)]
enum WorkOutcome {
    Completed(&'static str),
    Cancelled,
}

async fn work_with_cancel_token(
    work_ms: u64,
    mut cancel_rx: oneshot::Receiver<()>,
) -> WorkOutcome {
    tokio::select! {
        res = fake_io_work(work_ms) => WorkOutcome::Completed(res),
        _ = &mut cancel_rx => WorkOutcome::Cancelled,
    }
}

async fn cancel_beats_slow_work() -> bool {
    let (tx, rx) = oneshot::channel();
    let fut = work_with_cancel_token(500, rx);
    tokio::spawn(async move {
        sleep(Duration::from_millis(2)).await;
        let _ = tx.send(());
    });
    matches!(fut.await, WorkOutcome::Cancelled)
}

async fn work_finishes_if_not_cancelled() -> bool {
    let (_tx, rx) = oneshot::channel::<()>();
    matches!(
        work_with_cancel_token(3, rx).await,
        WorkOutcome::Completed("done")
    )
}

// ─────────────────────────────────────────────
// select 超时：到点视为取消
// ─────────────────────────────────────────────

async fn with_timeout<T>(
    fut: impl std::future::Future<Output = T>,
    timeout_ms: u64,
) -> Result<T, ()> {
    tokio::select! {
        v = fut => Ok(v),
        _ = sleep(Duration::from_millis(timeout_ms)) => Err(()),
    }
}

async fn timeout_cancels_slow() -> bool {
    with_timeout(sleep(Duration::from_millis(50)), 5)
        .await
        .is_err()
}

async fn run_tests() {
    assert!(abort_stops_progress().await);

    assert!(cancel_beats_slow_work().await);
    assert!(work_finishes_if_not_cancelled().await);

    assert!(timeout_cancels_slow().await);

    println!("✅ 异步取消所有测试用例通过！");
}

async fn bootstrap() {
    println!("\n\n************ L02 异步取消 ************\n");
    println!("（提示）取消 = 协作式；在 .await 边界检查信号或 abort\n");

    let aborted = abort_stops_progress().await;
    println!("abort 后计数未飙高？ {aborted}");

    let (tx, rx) = oneshot::channel();
    let work = tokio::spawn(work_with_cancel_token(200, rx));
    sleep(Duration::from_millis(3)).await;
    let _ = tx.send(());
    println!("提前取消结果: {:?}", work.await);

    println!(
        "慢任务超时？ {:?}",
        with_timeout(sleep(Duration::from_millis(30)), 5).await
    );
}

#[tokio::main]
async fn main() {
    bootstrap().await;
    run_tests().await;
}