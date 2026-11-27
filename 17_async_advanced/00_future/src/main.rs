/*
[主题]: Future 心智 —— async 背后那「可被轮询的状态机」
[难度]: Medium
[前置]: 07_async_tokio/00_async（会写 async/await）；不要求已懂 Pin / 手写 Future

一、主题说明
- **你已经会**：`async fn` + `.await` + Tokio —— 本课回答「`.await` 底下到底在干什么」。
- **`Future` trait**（标准库）：异步值的统一接口。核心只有 `poll`：
  - 返回 `Poll::Pending` →「还没好，先别往下走」；运行时会记住你，有进展再唤醒。
  - 返回 `Poll::Ready(v)` → 完成，`.await` 把 `v` 交回给你的 async 函数。
- **`async fn` 编译器产物**：一个匿名的、实现了 `Future` 的结构体（**状态机**），每个 `.await` 边界是一个状态。
- **Waker**：`poll` 时通过 `Context` 拿到；Future 在 `Pending` 时应注册「好了叫我」→ 典型是 I/O 就绪或定时器到点。

二、写法对比
1. 直观版 `ReadyFuture`：第一次 `poll` 就 `Ready`（对照「立刻完成」）。
2. 进阶版 `YieldThenReady`：先若干次 `Pending` + `wake`，最后用 `yield_now` 协作式让出（对照「多步才能完成」）。
3. 对照版：`async fn` 包一层，与手写 Future 行为一致（说明语法糖）。

三、真实用途
- 读 Tokio / async-book 源码时看到 `Poll`、`Pin<&mut Self>` 不再懵。
- 写流式协议、自定义 `Stream`、与 C++/嵌入式桥接时可能要手写 `Future`。
- 调优「为啥这个 future 从不 Ready」：看是不是少了 wake、或 poll 里阻塞了。

资料: https://rust-lang.github.io/async-book/
      https://doc.rust-lang.org/stable/book/ch17-00-async-await.html
      https://doc.rust-lang.org/std/future/trait.Future.html
      https://docs.rs/tokio/latest/tokio/
*/

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// ─────────────────────────────────────────────
// 直观版：立刻 Ready 的 Future（像「已经算完的结果」）
// ─────────────────────────────────────────────

struct ReadyFuture<T> {
    value: Option<T>,
}

impl<T> ReadyFuture<T> {
    fn new(value: T) -> Self {
        Self { value: Some(value) }
    }
}

impl<T> Future for ReadyFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        match this.value.take() {
            Some(v) => Poll::Ready(v),
            None => panic!("ReadyFuture 被 poll 了两次（违反 Future 契约）"),
        }
    }
}

// ─────────────────────────────────────────────
// 进阶版：需要多轮 poll + 协作式 yield 才 Ready
// ─────────────────────────────────────────────

/// 前 `pending_rounds` 次 poll 返回 Pending 并 wake 自己；之后返回 Ready。
/// 注意：在真实执行器里「立刻 wake」可能造成忙等；本课用极小次数 + Tokio 调度，仅教学。
struct YieldThenReady {
    pending_rounds: u32,
    output: u32,
}

impl YieldThenReady {
    fn new(pending_rounds: u32, output: u32) -> Self {
        Self {
            pending_rounds,
            output,
        }
    }
}

impl Future for YieldThenReady {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.pending_rounds > 0 {
            self.pending_rounds -= 1;
            // 告诉执行器：请稍后再 poll 我（有 wake 才能进展）
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(self.output)
        }
    }
}

/// 用 async 块表达「先让出两次，再返回值」—— 与状态机等价的心智模型
async fn async_yield_then_return(target: u32) -> u32 {
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;
    target
}

/// 演示：`.await` 会把多个 Future 串成一条链（编译器生成组合状态）
async fn chained_via_await() -> String {
    let n = ReadyFuture::new(10).await;
    let m = YieldThenReady::new(1, n + 5).await;
    format!("chain:{m}")
}

async fn run_tests() {
    assert_eq!(ReadyFuture::new(7).await, 7);

    assert_eq!(YieldThenReady::new(2, 99).await, 99);

    assert_eq!(async_yield_then_return(42).await, 42);

    assert_eq!(chained_via_await().await, "chain:15");

    // async fn 本身也是 Future：这里证明可以当普通 future 用 .await 驱动
    let fut = sum_two_async(3, 4);
    assert_eq!(fut.await, 7);

    println!("✅ Future 心智所有测试用例通过！");
}

async fn sum_two_async(a: u32, b: u32) -> u32 {
    tokio::task::yield_now().await;
    a + b
}

async fn bootstrap() {
    println!("\n\n************ L00 Future 心智 ************\n");
    println!("（提示）async fn = 返回匿名 Future；.await = 反复 poll 直到 Ready\n");

    println!(
        "ReadyFuture::new(100).await = {}",
        ReadyFuture::new(100).await
    );

    println!(
        "YieldThenReady(2 轮 Pending).await = {}",
        YieldThenReady::new(2, 200).await
    );

    println!(
        "chained_via_await = {}",
        chained_via_await().await
    );
}

#[tokio::main]
async fn main() {
    bootstrap().await;
    run_tests().await;
}