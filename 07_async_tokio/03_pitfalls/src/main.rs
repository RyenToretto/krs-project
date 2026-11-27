/*
[主题]: async 陷阱 —— 在运行时里做阻塞 vs spawn_blocking
[难度]: Hard
[前置]: 06_std_concurrency/00_thread、07_async_tokio/01_runtime

一、主题说明
- Tokio 用**少量线程**跑**大量** async 任务；任务应在 `.await` 时主动让出 CPU。
- **陷阱**：在 async 里调用 `std::thread::sleep`、重 CPU 循环、阻塞式磁盘 API → **占住 worker 线程**，其他任务全被拖慢甚至「饿死」。
- **正确方向**：
  1. 等待时间用 `tokio::time::sleep`（可唤醒、可让出）。
  2. 必须阻塞时用 `tokio::task::spawn_blocking` 丢到专用阻塞线程池。
- 本课 **不写** 几百毫秒的真阻塞在主 worker 上（避免 verify 变慢）；用 15ms 量级 + `Instant` 对比「挡路」与「spawn_blocking」。

二、写法对比
1. 反例（仅测试里短期演示）`blocking_sleep_in_async`：让别的并发 sleep 明显变慢。
2. 进阶版 `blocking_work_offloaded`：`spawn_blocking` 里 `thread::sleep`，async 侧只 await handle。

三、真实用途
- 调 legacy 同步 SDK、压缩大文件：spawn_blocking 或专用线程池。
- 数据库：生产用 async 驱动（sqlx）；同步驱动别直接堵在 Axum handler 里。
资料: https://tokio.rs/tokio/tutorial
      https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
      https://doc.rust-lang.org/stable/book/ch17-00-async-await.html
      https://www.youtube.com/@RustVideos
*/

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// 反例：在 async 任务里阻塞 worker（仅教学；时间刻意很短）
async fn blocking_sleep_in_async(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

/// 正例：阻塞段放进 spawn_blocking
async fn blocking_work_offloaded(ms: u64) -> Result<(), tokio::task::JoinError> {
    tokio::task::spawn_blocking(move || {
        thread::sleep(Duration::from_millis(ms));
        42u32
    })
    .await
    .map(|_| ())
}

/// 并发启动 n 个「应该很快完成」的 async sleep，测总耗时
async fn time_concurrent_light_tasks(n: usize, during: impl std::future::Future<Output = ()>) {
    let done = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::with_capacity(n);
    for _ in 0..n {
        let done = Arc::clone(&done);
        handles.push(tokio::spawn(async move {
            sleep(Duration::from_millis(20)).await;
            done.fetch_add(1, Ordering::SeqCst);
        }));
    }
    during.await;
    for h in handles {
        let _ = h.await;
    }
}

/// 对比：中间插一段「错误阻塞」会让轻量任务集体延期
async fn measure_delay_with_bad_block() -> u128 {
    let t = Instant::now();
    time_concurrent_light_tasks(4, async {
        blocking_sleep_in_async(15).await;
    })
    .await;
    t.elapsed().as_millis()
}

/// 对比：阻塞挪到 spawn_blocking，轻量任务仍能交错完成
async fn measure_delay_with_spawn_blocking() -> u128 {
    let t = Instant::now();
    time_concurrent_light_tasks(4, async {
        let _ = blocking_work_offloaded(15).await;
    })
    .await;
    t.elapsed().as_millis()
}

async fn cooperative_wait(ms: u64) {
    sleep(Duration::from_millis(ms)).await;
}

async fn run_tests() {
    let bad = measure_delay_with_bad_block().await;
    let good = measure_delay_with_spawn_blocking().await;

    // 反例会挡 worker，总时间往往更接近「串行挡路 + 子任务」；正例应更短（留余量防 CI 抖动）
    assert!(
        good <= bad.saturating_add(25),
        "spawn_blocking 场景不应比直接在 async 里阻塞更糟: good={good}ms bad={bad}ms"
    );

    cooperative_wait(5).await;

    println!("✅ async 阻塞陷阱 / spawn_blocking 所有测试用例通过！");
    println!("   （实测参考）bad_block≈{bad}ms  offload≈{good}ms");
}

async fn bootstrap() {
    println!("\n\n************ L03 async 陷阱 ************\n");
    println!("勿在 async 里长时间 thread::sleep；应用 tokio::time::sleep 或 spawn_blocking。\n");

    println!("合作式等待 30ms …");
    let t = Instant::now();
    cooperative_wait(30).await;
    println!("  耗时约 {} ms", t.elapsed().as_millis());

    let bad = measure_delay_with_bad_block().await;
    let good = measure_delay_with_spawn_blocking().await;
    println!("挡路阻塞拖累并发: ~{bad} ms");
    println!("spawn_blocking  offload: ~{good} ms");
}

#[tokio::main]
async fn main() {
    bootstrap().await;
    run_tests().await;
}