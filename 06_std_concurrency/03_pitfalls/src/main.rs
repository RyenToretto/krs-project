/*
[主题]: 并发陷阱 —— 死锁风险与可测规避手段
[难度]: Hard
[前置]: 02_mutex、01_channel

一、主题说明
- **死锁**：两个线程各拿一把锁，又互相等待对方手里的锁 → 永远卡住。
- 本课**绝不**在主流程里故意 `lock().unwrap()` 做成真死锁（会 hang 掉 verify）。
- 可测规避：① 全局固定加锁顺序；② `try_lock` 拿不到就释放已持锁、重试或报错。

二、写法对比
1. 陷阱示意（纸上）: 线程1 先锁 A 再锁 B，线程2 先 B 再 A —— 可能互相等待。
2. 进阶版: `transfer_ordered` 按指针地址排序先锁「小号」；`try_lock_twice` 演示探测。

三、真实用途
- 银行转账、双账户扣款：必须约定 lock ordering 或使用 try_lock + 退避。
- 生产里还会用 timeout lock、专用单线程执行器处理有依赖的共享状态。
资料: https://doc.rust-lang.org/book/ch16-03-shared-state.html
      https://doc.rust-lang.org/std/sync/struct.Mutex.html
      https://doc.rust-lang.org/std/sync/struct.Arc.html
      https://www.youtube.com/@RustVideos
*/

use std::cmp::Ordering;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

#[derive(Debug, PartialEq, Eq)]
enum TryLockProbe {
    BothAcquired,
    WouldBlock,
    Poisoned,
}
use std::thread;
use std::time::Duration;

type Account = Arc<Mutex<i32>>;

/// 按「锁的身份」排序：先锁地址较小者，打破「你等我、我等你」的环
fn lock_pair_ordered<'a>(
    a: &'a Account,
    b: &'a Account,
) -> (MutexGuard<'a, i32>, MutexGuard<'a, i32>) {
    let addr_a = Arc::as_ptr(a) as usize;
    let addr_b = Arc::as_ptr(b) as usize;
    match addr_a.cmp(&addr_b) {
        Ordering::Less => {
            let ga = a.lock().expect("poisoned");
            let gb = b.lock().expect("poisoned");
            (ga, gb)
        }
        Ordering::Greater => {
            let gb = b.lock().expect("poisoned");
            let ga = a.lock().expect("poisoned");
            (ga, gb)
        }
        Ordering::Equal => {
            let ga = a.lock().expect("poisoned");
            let gb = a.lock().expect("poisoned");
            (ga, gb)
        }
    }
}

/// 安全转账：固定顺序加锁，避免交叉等待
fn transfer_ordered(from: &Account, to: &Account, amount: i32) -> Result<(), &'static str> {
    if amount <= 0 {
        return Err("金额必须为正");
    }
    if Arc::ptr_eq(from, to) {
        return Err("转出与转入不能是同一账户");
    }
    let (mut first, mut second) = lock_pair_ordered(from, to);
    let (from_bal, to_bal) = if Arc::as_ptr(from) < Arc::as_ptr(to) {
        (&mut *first, &mut *second)
    } else {
        (&mut *second, &mut *first)
    };

    if *from_bal < amount {
        return Err("余额不足");
    }
    *from_bal -= amount;
    *to_bal += amount;
    Ok(())
}

/// 故意「错误顺序」的 try_lock 探测：不阻塞，能证明第二把锁暂时拿不到
fn try_lock_deadlock_probe(a: &Account, b: &Account) -> TryLockProbe {
    let g1 = match a.try_lock() {
        Ok(g) => g,
        Err(TryLockError::WouldBlock) => return TryLockProbe::WouldBlock,
        Err(TryLockError::Poisoned(_)) => return TryLockProbe::Poisoned,
    };
    // 若另一线程以相反顺序持锁，这里会 WouldBlock，而不是永远卡住
    match b.try_lock() {
        Ok(_g2) => {
            drop(g1);
            TryLockProbe::BothAcquired
        }
        Err(TryLockError::WouldBlock) => TryLockProbe::WouldBlock,
        Err(TryLockError::Poisoned(_)) => TryLockProbe::Poisoned,
    }
}

/// 多线程交叉转账压力测：有序加锁应始终完成（先充入足够余额，避免并发中「余额不足」误报）
fn stress_transfers(
    acc_a: Account,
    acc_b: Account,
    rounds: usize,
) -> (i32, i32) {
    let seed = 50_000_i32;
    *acc_a.lock().expect("poisoned") = seed;
    *acc_b.lock().expect("poisoned") = seed;
    let total_before = seed * 2;

    let half = rounds / 2;
    let a1 = Arc::clone(&acc_a);
    let b1 = Arc::clone(&acc_b);
    let a2 = Arc::clone(&acc_a);
    let b2 = Arc::clone(&acc_b);

    let h1 = thread::spawn(move || {
        for _ in 0..half {
            transfer_ordered(&a1, &b1, 1).unwrap();
        }
    });
    let h2 = thread::spawn(move || {
        for _ in 0..half {
            // 反向转账，但 lock_pair_ordered 保证顺序一致
            transfer_ordered(&b2, &a2, 1).unwrap();
        }
    });

    h1.join().expect("h1");
    h2.join().expect("h2");

    let va = *acc_a.lock().unwrap();
    let vb = *acc_b.lock().unwrap();
    debug_assert_eq!(va + vb, total_before, "总额应守恒");
    (va, vb)
}

fn run_tests() {
    let alice: Account = Arc::new(Mutex::new(100));
    let bob: Account = Arc::new(Mutex::new(50));

    transfer_ordered(&alice, &bob, 30).unwrap();
    assert_eq!(*alice.lock().unwrap(), 70);
    assert_eq!(*bob.lock().unwrap(), 80);

    assert!(transfer_ordered(&alice, &bob, 999).is_err());

    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(0));
    let (va, vb) = stress_transfers(Arc::clone(&a), Arc::clone(&b), 200);
    assert_eq!(va + vb, 100_000, "总额应保持守恒");

    // try_lock 探测：一个线程占满两把锁时，另一线程应得到 WouldBlock
    let x = Arc::new(Mutex::new(1));
    let y = Arc::new(Mutex::new(2));
    let x_hold = Arc::clone(&x);
    let y_hold = Arc::clone(&y);
    let blocker = thread::spawn(move || {
        let _gx = x_hold.lock().unwrap();
        let _gy = y_hold.lock().unwrap();
        thread::sleep(Duration::from_millis(80));
    });
    thread::sleep(Duration::from_millis(10));
    // 与 blocker 抢相反顺序：try_lock 应快速失败至少一次
    let mut saw_would_block = false;
    for _ in 0..20 {
        match try_lock_deadlock_probe(&y, &x) {
            TryLockProbe::BothAcquired => {}
            TryLockProbe::WouldBlock => {
                saw_would_block = true;
                break;
            }
            TryLockProbe::Poisoned => panic!("不应 poison"),
        }
        thread::sleep(Duration::from_millis(2));
    }
    blocker.join().unwrap();
    assert!(
        saw_would_block,
        "应用 try_lock 观测到至少一次 WouldBlock（说明规避阻塞检测有效）"
    );

    println!("✅ 并发陷阱与规避 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 并发陷阱与规避 ************\n");

    let alice: Account = Arc::new(Mutex::new(500));
    let bob: Account = Arc::new(Mutex::new(100));

    match transfer_ordered(&alice, &bob, 120) {
        Ok(()) => println!(
            "转账后 Alice={}, Bob={}",
            *alice.lock().unwrap(),
            *bob.lock().unwrap()
        ),
        Err(e) => println!("转账失败: {e}"),
    }

    let stress_a: Account = Arc::new(Mutex::new(0));
    let stress_b: Account = Arc::new(Mutex::new(0));
    let (va, vb) = stress_transfers(stress_a, stress_b, 50);
    println!("交叉转账后合计守恒检查: 两账户余额和 = {}", va + vb);

    println!("提示: 生产环境避免嵌套多把 Mutex::lock；优先 channel 或单线程 executor。");

    run_tests();
}

fn main() {
    bootstrap();
}