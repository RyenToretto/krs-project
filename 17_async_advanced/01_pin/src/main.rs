/*
[主题]: Pin 心智 —— 为啥 async 状态机「钉」在原地不能乱动
[难度]: Medium
[前置]: 17_async_advanced/00_future；01_ownership_borrowing

一、主题说明
- **问题从哪来**：`async fn` 的状态机里，字段之间可能互相引用（**自引用** struct）。
  若 Future 被 **move** 到新地址，里面的裸指针还指着旧地址 → 未定义行为。
- **`Pin<P>`**：承诺「这块内存上的值不会先被 **移动** 再被使用」（除非类型是 `Unpin`）。
- **`Future::poll` 签名**：`fn poll(self: Pin<&mut Self>, ...)` —— 执行器只能在你钉住的引用上推进状态机。
- **`Unpin`**：默认几乎所有类型都是；**不是** `Unpin` 的类型（含编译器生成的部分 async 状态）必须用 `Pin`。
- **小白记法**：不用自己给每个 async 打 Pin；写 `async fn` 时编译器 + Tokio 替你搞定。学 Pin 是为了读懂 trait、写自引用结构、接 Stream。

二、写法对比
1. 直观版 `pin_fixed_string`：`Pin<Box<T>>` 固定堆上对象，演示「钉住后可以安全读」。
2. 进阶版 `SelfReferential`：结构体里指针指向自己的字段；**必须** `Pin<Box<...>>` + 初始化后不再移动。
3. `poll_pinned_future`：手写 `Future` 在 `Pin<&mut Self>` 上 `poll`（与标准库一致）。

三、真实用途
- 实现 `Stream`、解析器状态机、环形缓冲异步读。
- 理解「为啥不能把未 Pin 的 `!Unpin` future 随便 `mem::swap`」。
- 读 `pin-project` / `tokio::pin!` 宏时知道它在保什么证。

资料: https://doc.rust-lang.org/std/pin/struct.Pin.html
      https://rust-lang.github.io/async-book/
      https://docs.rs/tokio/latest/tokio/
      https://doc.rust-lang.org/stable/book/ch17-00-async-await.html
*/

use std::future::Future;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::task::{Context, Poll};

// ─────────────────────────────────────────────
// 直观版：Pin<Box<T>> —— 堆上分配 + 钉住，之后按引用用
// ─────────────────────────────────────────────

fn pin_fixed_string(text: &str) -> Pin<Box<String>> {
    let pinned = Box::pin(text.to_string());
    // Box::pin 返回 Pin<Box<T>>，T 在堆地址不变（除非整个 Box 被 drop）
    pinned
}

fn read_pinned(msg: Pin<&str>) -> usize {
    msg.len()
}

// ─────────────────────────────────────────────
// 进阶版：自引用结构 —— 没有 Pin 就不能安全存在
// ─────────────────────────────────────────────

struct SelfReferential {
    data: String,
  /// 指向 `data` 的指针；要求 Self 在内存中不移动
    self_ptr: *const String,
    _pin: PhantomPinned,
}

impl SelfReferential {
    /// 返回已钉住的 Box；初始化指针时必须用 `get_unchecked_mut`（Pin 还没建立前的唯一窗口）
    fn new(data: String) -> Pin<Box<Self>> {
        let mut boxed = Box::pin(SelfReferential {
            data,
            self_ptr: std::ptr::null(),
            _pin: PhantomPinned,
        });
        let data_addr = &boxed.data as *const String;
        unsafe {
            let inner = boxed.as_mut().get_unchecked_mut();
            inner.self_ptr = data_addr;
        }
        boxed
    }

    fn pointer_still_valid(this: Pin<&Self>) -> bool {
        this.self_ptr == &this.data as *const String
    }

    fn data_len(this: Pin<&Self>) -> usize {
        this.data.len()
    }
}

// ─────────────────────────────────────────────
// 手写 Future：poll 接收 Pin<&mut Self>（与 std::Future 一致）
// ─────────────────────────────────────────────

struct StepOnce {
    done: bool,
    _pin: PhantomPinned,
}

impl StepOnce {
    fn new() -> Pin<Box<Self>> {
        Box::pin(StepOnce {
            done: false,
            _pin: PhantomPinned,
        })
    }
}

impl Future for StepOnce {
    type Output = &'static str;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.done {
            Poll::Ready("pinned-future-done")
        } else {
            this.done = true;
            Poll::Pending
        }
    }
}

/// async 块生成的 Future 由编译器处理 Pin；这里包一层展示用法
async fn async_after_pin_demo() -> usize {
    let pinned = pin_fixed_string("hello-pin");
    // Pin<&String> → 取内容长度（堆地址不变）
    pinned.as_ref().len()
}

async fn run_tests() {
    let p = pin_fixed_string("rust");
    assert_eq!(*p, "rust");
    let s = "async".to_string();
    let p: Pin<&str> = Pin::new(s.as_str());
    assert_eq!(read_pinned(p), 5);

    let sr = SelfReferential::new("self-ref".into());
    assert!(SelfReferential::pointer_still_valid(sr.as_ref()));
    assert_eq!(SelfReferential::data_len(sr.as_ref()), 8);

    let mut fut = StepOnce::new();
    assert!(matches!(fut.as_mut().poll(&mut noop_waker()), Poll::Pending));
    assert!(matches!(
        fut.as_mut().poll(&mut noop_waker()),
        Poll::Ready("pinned-future-done")
    ));

    assert_eq!(async_after_pin_demo().await, 9);

    println!("✅ Pin 心智所有测试用例通过！");
}

fn noop_waker() -> std::task::Context<'static> {
    use std::sync::OnceLock;
    use std::task::{Context, RawWaker, RawWakerVTable, Waker};

    static VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE),
        |_| {},
        |_| {},
        |_| {},
    );

    static WAKER: OnceLock<Waker> = OnceLock::new();
    let waker = WAKER.get_or_init(|| unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) });
    Context::from_waker(waker)
}

async fn bootstrap() {
    println!("\n\n************ L01 Pin 心智 ************\n");
    println!("（提示）Pin ≠ 指针；Pin = 「这块内存里的值不许被搬家」\n");

    let sr = SelfReferential::new("演示".into());
    println!(
        "自引用指针仍有效？ {}",
        SelfReferential::pointer_still_valid(sr.as_ref())
    );

    let mut step = StepOnce::new();
    let _ = step.as_mut().poll(&mut noop_waker());
    println!(
        "StepOnce 第二次 poll = {:?}",
        step.as_mut().poll(&mut noop_waker())
    );
}

#[tokio::main]
async fn main() {
    bootstrap().await;
    run_tests().await;
}