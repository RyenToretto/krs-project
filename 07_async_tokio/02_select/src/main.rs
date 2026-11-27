/*
[主题]: select! 与 join! —— 多路等待与竞速
[难度]: Medium
[前置]: 07_async_tokio/01_runtime

一、主题说明
- **`tokio::join!(a, b, …)`**：等**全部**完成，返回所有结果（并发推进，适合「都要」）。
- **`tokio::select!`**：等多路里**先完成的那一路**（可设 `biased` 等高级用法；本课用基础分支）。
- **`try_join!`**：类似 join，但任一路 `Err` 就整体短路（常用于多个 fallible async）。

二、写法对比
1. 直观版：固定先等 A 再等 B（没有竞速）。
2. 进阶版：`select!` 在「快任务」与「慢任务」里取先完成的；`join!` 并行跑完两个计数器。

三、真实用途
- 请求带超时：`select! { res = call() => ..., _ = sleep(timeout) => timeout }`（本课用毫秒级演示）。
- 同时读 socket + 等关闭信号：典型 select 场景。
资料: https://docs.rs/tokio/latest/tokio/macro.select.html
      https://docs.rs/tokio/latest/tokio/macro.join.html
      https://tokio.rs/tokio/tutorial
      https://rust-lang.github.io/async-book/
      https://www.youtube.com/@RustVideos
*/

use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, PartialEq, Eq)]
enum RaceWinner {
    Fast,
    Slow,
}

/// 先完成者获胜（模拟「哪个服务先响应」）
async fn race_fast_vs_slow() -> RaceWinner {
    tokio::select! {
        _ = sleep(Duration::from_millis(10)) => RaceWinner::Fast,
        _ = sleep(Duration::from_millis(40)) => RaceWinner::Slow,
    }
}

/// 两个计数器并发累加
async fn ticker(name: &str, steps: u32, step_ms: u64) -> String {
    let mut n = 0u32;
    for _ in 0..steps {
        sleep(Duration::from_millis(step_ms)).await;
        n += 1;
    }
    format!("{name}:{n}")
}

/// 直观版：顺序跑两个 ticker（慢）
async fn tickers_sequential() -> (String, String) {
    let a = ticker("A", 3, 5).await;
    let b = ticker("B", 3, 5).await;
    (a, b)
}

/// 进阶版：join! 同时跑
async fn tickers_parallel() -> (String, String) {
    tokio::join!(ticker("A", 3, 5), ticker("B", 3, 5))
}

/// try_join! 演示：两路都 Ok 才成功
async fn may_fail(ok: bool) -> Result<u32, &'static str> {
    sleep(Duration::from_millis(1)).await;
    if ok {
        Ok(1)
    } else {
        Err("fail")
    }
}

async fn try_both(ok_a: bool, ok_b: bool) -> Result<(u32, u32), &'static str> {
    tokio::try_join!(may_fail(ok_a), may_fail(ok_b))
}

async fn run_tests() {
    assert_eq!(race_fast_vs_slow().await, RaceWinner::Fast);

    let (sa, sb) = tickers_sequential().await;
    assert_eq!(sa, "A:3");
    assert_eq!(sb, "B:3");

    let (pa, pb) = tickers_parallel().await;
    assert_eq!(pa, "A:3");
    assert_eq!(pb, "B:3");

    assert_eq!(try_both(true, true).await, Ok((1, 1)));
    assert!(try_both(true, false).await.is_err());

    println!("✅ select! / join! 所有测试用例通过！");
}

async fn bootstrap() {
    println!("\n\n************ L02 select! / join! ************\n");

    println!("竞速: {:?}", race_fast_vs_slow().await);
    let (a, b) = tickers_parallel().await;
    println!("并行 ticker: {a}, {b}");
}

#[tokio::main]
async fn main() {
    bootstrap().await;
    run_tests().await;
}