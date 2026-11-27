/*
[主题]: Mutex + Arc —— 多线程共享可变状态
[难度]: Medium → Hard
[前置]: 00_thread、01_channel（消息传递是对照方案）

一、主题说明
- 多个线程不能同时持有 `&mut T`；`Mutex<T>` 在运行时保证「同一时刻只有一个锁守卫」。
- `Arc<T>` = 原子引用计数，多线程克隆**共享所有权**；常写作 `Arc<Mutex<T>>`。
- `lock()` 可能阻塞；持锁期间别做耗时 IO，否则其他线程全等（活锁/性能坑）。

二、写法对比
1. 直观版: 单线程 `RefCell` 式思维，自己用 `Mutex` 包一层计数器。
2. 进阶版: `Arc<Mutex<Vec<_>>>`，多线程 `clone` Arc 后各自 `lock` 追加。

三、真实用途
- 进程内全局计数、连接池元数据、内存缓存（小规模）；大规模优先 channel 或分片锁。
- 读多写少时会换 `RwLock`（本阶段先掌握 Mutex）。
资料: https://doc.rust-lang.org/book/ch16-03-shared-state.html
      https://doc.rust-lang.org/std/sync/struct.Mutex.html
      https://doc.rust-lang.org/std/sync/struct.Arc.html
      https://www.youtube.com/@RustVideos
*/

use std::sync::{Arc, Mutex};
use std::thread;

/// 直观版：单线程递增
fn increment_naive(start: i32, steps: u32) -> i32 {
    let mut n = start;
    for _ in 0..steps {
        n += 1;
    }
    n
}

/// 进阶版：Arc<Mutex<i32>>，多线程各加一部分
fn increment_parallel_better(start: i32, steps_per_thread: u32, threads: usize) -> i32 {
    let counter = Arc::new(Mutex::new(start));
    let mut handles = Vec::with_capacity(threads);

    for _ in 0..threads {
        let c = Arc::clone(&counter);
        let h = thread::spawn(move || {
            let mut guard = c.lock().expect("mutex poisoned");
            for _ in 0..steps_per_thread {
                *guard += 1;
            }
        });
        handles.push(h);
    }

    for h in handles {
        h.join().expect("join failed");
    }

    let final_val = *counter.lock().expect("mutex poisoned");
    final_val
}

/// 多线程往共享 Vec 追加（每线程追加若干条带 id 的字符串）
fn collect_labels(worker_count: usize, per_worker: usize) -> Vec<String> {
    let bucket = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut handles = Vec::new();

    for wid in 0..worker_count {
        let b = Arc::clone(&bucket);
        let h = thread::spawn(move || {
            let mut guard = b.lock().expect("mutex poisoned");
            for seq in 0..per_worker {
                guard.push(format!("w{wid}-s{seq}"));
            }
        });
        handles.push(h);
    }

    for h in handles {
        h.join().expect("join failed");
    }

    let mut v = bucket.lock().expect("mutex poisoned").clone();
    v.sort_unstable();
    v
}

fn run_tests() {
    assert_eq!(increment_naive(0, 10), 10);
    assert_eq!(increment_parallel_better(0, 25, 4), 100);

    let labels = collect_labels(3, 2);
    assert_eq!(labels.len(), 6);
    assert!(labels.contains(&"w0-s0".to_string()));
    assert!(labels.contains(&"w2-s1".to_string()));

    let data = Arc::new(Mutex::new(5));
    let c1 = Arc::clone(&data);
    let c2 = Arc::clone(&data);
    let h1 = thread::spawn(move || {
        *c1.lock().unwrap() += 10;
    });
    let h2 = thread::spawn(move || {
        *c2.lock().unwrap() *= 2;
    });
    h1.join().unwrap();
    h2.join().unwrap();
    // 顺序不确定，只断言在合理范围
    let final_val = *data.lock().unwrap();
    assert!(final_val == 30 || final_val == 20, "unexpected {final_val}");

    println!("✅ Mutex / Arc 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Mutex + Arc ************\n");

    println!(
        "单线程 +{}: {}",
        5,
        increment_naive(100, 5)
    );
    println!(
        "4 线程各 +25: {}",
        increment_parallel_better(0, 25, 4)
    );

    println!(
        "共享 Vec 收集: {:?}",
        collect_labels(2, 3)
    );

    run_tests();
}

fn main() {
    bootstrap();
}