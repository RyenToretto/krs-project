/*
[主题]: async / await 入门 —— 异步函数与「可等待」的任务
[难度]: Easy
[前置]: 06_std_concurrency/00_thread（知道「并发」概念即可）；不要求已懂 Future

一、主题说明
- **同步**：函数一口气跑完才返回；等待磁盘/网络时，线程往往被白白占住。
- **async fn**：返回的是「将来会算完的活」（在 Rust 里类型叫 `Future`），**不会一调用就立刻跑完**。
- **`.await`**：在 async 函数里「挂起自己」，把执行权交给 **运行时（runtime）**，等这件事好了再继续。
- 本课用 `#[tokio::main]`：帮你建好 Tokio 运行时，并让 `main` 能写 `async` + `.await`（小白先记：**没有 runtime，async 跑不起来**）。

二、写法对比
1. 直观版 `fetch_sequential`：任务 A 做完再做 B，总耗时 ≈ 两者相加。
2. 进阶版 `fetch_together`：用 `tokio::join!` 同时等两个 async，总耗时 ≈ 较慢那个（本课用 `sleep` 模拟 I/O，**不连网**）。

三、真实用途
- Web 请求：一边查数据库一边调别的服务 → 别傻等，用 async 把等待叠起来。
- Axum 里的 handler 几乎都是 `async fn`；背后就是 Tokio 在调度。
资料: https://doc.rust-lang.org/stable/book/ch17-00-async-await.html
      https://rust-lang.github.io/async-book/
      https://tokio.rs/tokio/tutorial
      https://docs.rs/tokio/latest/tokio/
      https://tokio.rs/
      https://www.youtube.com/@RustVideos
*/

use std::time::{Duration, Instant};
use tokio::time::sleep;

/// 假装「从远端取数据」：用 sleep 代替真网络（离线可测）
async fn fake_fetch(label: &str, ms: u64) -> String {
    sleep(Duration::from_millis(ms)).await;
    format!("数据-{label}")
}

/// 直观版：一个接一个 await
async fn fetch_sequential() -> (String, String) {
    let a = fake_fetch("甲", 25).await;
    let b = fake_fetch("乙", 25).await;
    (a, b)
}

/// 进阶版：两个 future 一起推进（并发等待）
async fn fetch_together() -> (String, String) {
    let (a, b) = tokio::join!(fake_fetch("甲", 25), fake_fetch("乙", 25));
    (a, b)
}

/// 演示：普通函数里不能 `.await`，必须在外层 async 或 runtime 里
async fn double_via_await(x: u32) -> u32 {
    sleep(Duration::from_millis(1)).await;
    x * 2
}

async fn run_tests() {
    let (sa, sb) = fetch_sequential().await;
    assert_eq!(sa, "数据-甲");
    assert_eq!(sb, "数据-乙");

    let (pa, pb) = fetch_together().await;
    assert_eq!(pa, "数据-甲");
    assert_eq!(pb, "数据-乙");

    // 并发应明显快于顺序（留一点调度抖动余量）
    let t0 = Instant::now();
    let _ = fetch_sequential().await;
    let seq_ms = t0.elapsed().as_millis();

    let t1 = Instant::now();
    let _ = fetch_together().await;
    let par_ms = t1.elapsed().as_millis();

    assert!(
        par_ms < seq_ms,
        "并发等待应比顺序快: parallel={par_ms}ms sequential={seq_ms}ms"
    );

    assert_eq!(double_via_await(21).await, 42);

    println!("✅ async/await 基础所有测试用例通过！");
}

async fn bootstrap() {
    println!("\n\n************ L00 async / await ************\n");
    println!("（小白提示）async fn 要配合 runtime；本课用 #[tokio::main]\n");

    println!("顺序 fake_fetch …");
    let t = Instant::now();
    let (a, b) = fetch_sequential().await;
    println!("  结果: {a}, {b}，耗时约 {} ms", t.elapsed().as_millis());

    println!("并发 tokio::join! …");
    let t = Instant::now();
    let (a, b) = fetch_together().await;
    println!("  结果: {a}, {b}，耗时约 {} ms", t.elapsed().as_millis());

    println!("double_via_await(10) = {}", double_via_await(10).await);
}

#[tokio::main]
async fn main() {
    bootstrap().await;
    run_tests().await;
}