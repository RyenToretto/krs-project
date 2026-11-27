/*
[主题]: std::thread —— 创建线程、join 收回结果
[难度]: Medium
[前置]: 01_ownership_borrowing（move 闭包）、03_error_collections

一、主题说明
- `thread::spawn` 启动新 OS 线程；`JoinHandle::join()` 阻塞等待子线程结束并拿回返回值。
- 传给 `spawn` 的闭包常写 `move`，把外层变量**所有权**搬进子线程（符合 Rust 「一线程一份数据」的直觉）。
- `join` 返回 `Result<T, Box<dyn Any + Send>>`：子线程 panic 时外层拿到 `Err`，不要忽略。

二、写法对比
1. 直观版: 单线程顺序算完再汇总。
2. 进阶版: 多线程分片计算，`join` 合并结果（仍只用 std）。

三、真实用途
- CPU 密集任务分块（哈希、压缩）在同步代码里仍常见。
- 理解 thread 后再学 Tokio：`spawn_blocking` 就是把阻塞工作丢到线程池。
资料: https://doc.rust-lang.org/book/ch16-00-concurrency.html
      https://doc.rust-lang.org/book/ch16-01-threads.html
      https://doc.rust-lang.org/std/thread/
      https://www.youtube.com/@RustVideos
*/

use std::thread;

/// 直观版：主线程自己循环求和
fn sum_naive(nums: &[i32]) -> i32 {
    nums.iter().sum()
}

/// 进阶版：拆成两半，两个 `spawn` + `move` 闭包，各自算一半再相加
fn sum_parallel_better(nums: &[i32]) -> i32 {
    let mid = nums.len() / 2;
    let left = nums[..mid].to_vec(); // 拷贝一份，便于 move 进线程
    let right = nums[mid..].to_vec();

    let h_left = thread::spawn(move || left.iter().sum::<i32>());
    let h_right = thread::spawn(move || right.iter().sum::<i32>());

    let a = h_left.join().expect("左半区线程 panic");
    let b = h_right.join().expect("右半区线程 panic");
    a + b
}

/// 演示：子线程通过 move 拿走 `String` 的所有权
fn greet_in_thread(name: String) -> thread::JoinHandle<String> {
    thread::spawn(move || format!("你好，{name}（来自子线程）"))
}

/// 批量 spawn，收集全部 join 结果
fn map_squares(nums: Vec<i32>) -> Vec<i32> {
    let handles: Vec<_> = nums
        .into_iter()
        .map(|n| thread::spawn(move || n * n))
        .collect();
    handles
        .into_iter()
        .map(|h| h.join().expect("平方线程 panic"))
        .collect()
}

fn run_tests() {
    let data: Vec<i32> = (1..=100).collect();
    assert_eq!(sum_naive(&data), 5050);
    assert_eq!(sum_parallel_better(&data), 5050);

    let handle = greet_in_thread("Rust 学习者".to_string());
    let msg = handle.join().unwrap();
    assert!(msg.contains("Rust 学习者"));
    assert!(msg.contains("子线程"));

    assert_eq!(map_squares(vec![1, 2, 3, 4]), vec![1, 4, 9, 16]);

    let h = thread::spawn(|| 42);
    assert_eq!(h.join().unwrap(), 42);

    println!("✅ std::thread 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 std::thread ************\n");

    let numbers: Vec<i32> = (1..=20).collect();
    println!(
        "顺序求和 1..=20 = {}",
        sum_naive(&numbers)
    );
    println!(
        "双线程求和 1..=20 = {}",
        sum_parallel_better(&numbers)
    );

    let h = greet_in_thread("krs-project".to_string());
    println!("{}", h.join().unwrap());

    println!(
        "多线程平方: {:?}",
        map_squares(vec![2, 3, 5, 8])
    );

    run_tests();
}

fn main() {
    bootstrap();
}