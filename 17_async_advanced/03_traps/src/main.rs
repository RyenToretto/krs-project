/*
[主题]: 异步进阶陷阱 —— 生产里最容易踩的坑
[难度]: Hard
[前置]: 07_async_tokio/03_pitfalls、17_async_advanced/00_future～02_cancel

一、主题说明
- **持锁跨 await**：`std::sync::Mutex` 在 async 里若锁着去 `.await` → 别的任务可能同线程要锁 → **死锁或长时间饥饿**。
  → 用 **`tokio::sync::Mutex`**（异步锁，await 时放锁）或缩小锁范围。
- **`spawn` 的 `'static`**：传入闭包的数据必须活得够久；`&引用` 往往过不了编译（好事）。
- **`select!` 取消分支后别再用被取消分支的 Future**（部分 branch 被 drop，可能半初始化）。
- **忘记 `.await` JoinHandle**：任务悄悄跑；测试里用 `JoinHandle` 必须 await 或 abort。
- **在 async 里阻塞 worker**：`thread::sleep` / 重 CPU（07 讲过）；本课强调与 **锁** 组合时的叠加危害。

二、写法对比
1. 反例（极短阻塞 + 锁）`std_sync_mutex_across_yield`：演示延迟放大（毫秒级）。
2. 正例 `tokio_mutex_then_await`：先算完、释放锁，再 await。
3. `select_drop_loser`：`select!` 后只用胜者分支的数据（对照 canceled 变量规则）。

三、真实用途
- Code review Axum handler：见 `std::sync::Mutex` + `.await` 要警惕。
- 高 QPS 下锁竞争 + 长 await → P99 爆炸。

资料: https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html
      https://docs.rs/tokio/latest/tokio/macro.select.html
      https://rust-lang.github.io/async-book/
      https://doc.rust-lang.org/stable/book/ch17-00-async-await.html
      https://docs.rs/tokio/latest/tokio/
*/

use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::Mutex as TokioMutex;
use tokio::time::sleep;

// ─────────────────────────────────────────────
// 反例：单任务内 std Mutex 持有期间 await
// （不能放进 tokio::spawn：MutexGuard 非 Send；但单任务里这样写仍会长时间占锁）
// 并发场景下第二个任务会在 `.lock()` 上阻塞 worker → 见过 07_pitfalls + 本课组合
// ─────────────────────────────────────────────

async fn std_mutex_held_across_await_ms() -> u128 {
    let mtx = StdMutex::new(0u32);
    let t0 = Instant::now();
    let mut guard = mtx.lock().unwrap();
    sleep(Duration::from_millis(8)).await; // guard 仍存活 → 锁未释放
    *guard = 1;
    drop(guard);
    t0.elapsed().as_millis()
}

// ─────────────────────────────────────────────
// 正例：缩小锁范围 + tokio Mutex
// ─────────────────────────────────────────────

async fn tokio_mutex_then_await() -> u128 {
    let mtx = Arc::new(TokioMutex::new(0u32));
    let t0 = Instant::now();

    let m1 = Arc::clone(&mtx);
    let h1 = tokio::spawn(async move {
        let mut g = m1.lock().await;
        *g += 1;
        drop(g); // 先放锁
        sleep(Duration::from_millis(8)).await;
    });

    let m2 = Arc::clone(&mtx);
    let h2 = tokio::spawn(async move {
        sleep(Duration::from_millis(1)).await;
        let _g = m2.lock().await;
    });

    let _ = tokio::join!(h1, h2);
    t0.elapsed().as_millis()
}

// ─────────────────────────────────────────────
// select! 后只用胜者 —— 避免被取消分支状态
// ─────────────────────────────────────────────

#[derive(Debug)]
enum Pick {
    Fast(u32),
    Slow(()),
}

async fn select_keep_winner() -> Pick {
    tokio::select! {
        v = fast_counter() => Pick::Fast(v),
        _ = slow_counter() => Pick::Slow(()),
    }
}

async fn fast_counter() -> u32 {
    sleep(Duration::from_millis(2)).await;
    1
}

async fn slow_counter() -> u32 {
    sleep(Duration::from_millis(40)).await;
    2
}

// ─────────────────────────────────────────────
// spawn 必须 await：否则测试无法断言副作用完成
// ─────────────────────────────────────────────

async fn increment_with_spawn(arc: Arc<TokioMutex<u32>>) -> u32 {
    let a = Arc::clone(&arc);
    let handle = tokio::spawn(async move {
        let mut g = a.lock().await;
        *g += 10;
    });
    handle.await.unwrap();
    let g = arc.lock().await;
    *g
}

async fn run_tests() {
    let bad_ms = std_mutex_held_across_await_ms().await;
    assert!(bad_ms >= 6, "持锁 await 会占满至少 sleep 时长: {bad_ms}ms");

    let good_ms = tokio_mutex_then_await().await;
    assert!(
        good_ms < 25,
        "tokio mutex 放锁后再 await，两任务应能交错: {good_ms}ms"
    );

    match select_keep_winner().await {
        Pick::Fast(1) => {}
        other => panic!("select 应优先 fast: {:?}", other),
    }

    let ctr = Arc::new(TokioMutex::new(0u32));
    assert_eq!(increment_with_spawn(ctr).await, 10);

    println!("✅ 异步进阶陷阱所有测试用例通过！");
}

async fn bootstrap() {
    println!("\n\n************ L03 异步进阶陷阱 ************\n");
    println!("（提示）async 里优先 tokio::sync::Mutex；别持 std 锁 await\n");

    let bad = std_mutex_held_across_await_ms().await;
    let good = tokio_mutex_then_await().await;
    println!("单任务持 std 锁跨 await ~{bad}ms；tokio 锁先放再 await 并发 ~{good}ms");

    let pick = select_keep_winner().await;
    println!("select 胜者: {:?}", pick);
}

#[tokio::main]
async fn main() {
    bootstrap().await;
    run_tests().await;
}