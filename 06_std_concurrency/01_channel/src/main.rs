/*
[主题]: mpsc channel —— 线程间传消息（所有权转移）
[难度]: Medium
[前置]: 00_thread、01_ownership_borrowing

一、主题说明
- `mpsc` = multiple producer, single consumer：多个发送端、一个接收端。
- `send` 会把值**移动**到通道里；接收端 `recv` 拿到所有权。
- 发送端全部 drop 后，`recv` 会得到 `Err`（通道关闭），可用来判断「不会再有消息」。

二、写法对比
1. 直观版: 单线程管道函数链，模拟「生产 → 消费」数据流。
2. 进阶版: 多生产者 `thread::spawn` + 克隆 `Sender`，主线程 `recv` 汇总。

三、真实用途
- 工作队列：后台线程干活，主线程收结果（类似简易任务队列）。
- 日志/指标聚合：多 writer 推事件，单消费者落盘（生产里常见 bounded channel）。
资料: https://doc.rust-lang.org/book/ch16-02-message-passing.html
      https://doc.rust-lang.org/std/sync/mpsc/
      https://doc.rust-lang.org/book/ch16-00-concurrency.html
      https://www.youtube.com/@RustVideos
*/

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// 直观版：不用线程，只是在函数里「装瓶」再「倒出来」
fn pipeline_naive(items: Vec<i32>) -> Vec<i32> {
    items.into_iter().map(|x| x * 2).collect()
}

/// 进阶版：通道 + 多生产者
fn pipeline_channel_better(items: Vec<i32>, worker_count: usize) -> Vec<i32> {
    let (tx, rx) = mpsc::channel::<i32>();

    let chunk = (items.len() + worker_count - 1) / worker_count.max(1);
    for part in items.chunks(chunk.max(1)) {
        let tx_clone = tx.clone();
        let batch: Vec<i32> = part.to_vec();
        thread::spawn(move || {
            for x in batch {
                // send 失败说明接收端已关，正常退出即可
                let _ = tx_clone.send(x * 2);
            }
        });
    }
    // 主发送端 drop：子线程手里的 Sender 是全部分支，最后一个生产者 drop 后通道才关
    drop(tx);

    let mut out = Vec::new();
    while let Ok(v) = rx.recv() {
        out.push(v);
    }
    out.sort_unstable();
    out
}

/// 带超时的 recv：避免测试里傻等（本课演示用；生产用 bounded + 明确协议）
fn recv_with_timeout(rx: &mpsc::Receiver<i32>, ms: u64) -> Option<i32> {
    rx.recv_timeout(Duration::from_millis(ms)).ok()
}

/// 单生产者单消费者最小示例
fn ping_pong(value: i32) -> i32 {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        tx.send(value + 1).unwrap();
    });
    rx.recv().unwrap()
}

fn run_tests() {
    let input: Vec<i32> = (1..=10).collect();
    let naive = pipeline_naive(input.clone());
    assert_eq!(naive, (1..=10).map(|x| x * 2).collect::<Vec<_>>());

    let via_ch = pipeline_channel_better(input, 3);
    let mut expected: Vec<i32> = (1..=10).map(|x| x * 2).collect();
    expected.sort_unstable();
    assert_eq!(via_ch, expected);

    assert_eq!(ping_pong(41), 42);

    let (tx, rx) = mpsc::channel::<String>();
    tx.send("hello".to_string()).unwrap();
    drop(tx);
    assert_eq!(rx.recv().unwrap(), "hello");
    assert!(rx.recv().is_err(), "发送端全关后 recv 应为 Err");

    let (tx2, rx2) = mpsc::sync_channel(1);
    tx2.send(7).unwrap();
    assert_eq!(recv_with_timeout(&rx2, 50), Some(7));

    println!("✅ mpsc channel 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 mpsc channel ************\n");

    let items: Vec<i32> = vec![3, 1, 4, 1, 5, 9];
    println!("单线程管道: {:?}", pipeline_naive(items.clone()));
    println!(
        "多线程 channel: {:?}",
        pipeline_channel_better(items, 2)
    );

    println!("ping_pong: {}", ping_pong(99));

    run_tests();
}

fn main() {
    bootstrap();
}