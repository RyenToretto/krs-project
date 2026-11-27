/*
[主题]: 基准心智 —— `Instant` 粗测与 Criterion 正式微基准的分工
[难度]: Medium
[前置]: `13_testing/00_unit`（表驱动）、`15_project_capstone` 建议

一、主题说明
- **性能问题先测量再改**：没数据就优化，往往优化错热点。
- 本课只用 **std::time::Instant** 做毫秒级、可重复的「教学粗测」；正式项目里把同一函数迁入 **Criterion**（统计、离群值、HTML 报告）。
- 粗测陷阱：第一次跑含「冷启动」；只测一次会被抖动骗了；Debug 与 Release 差一个数量级——本仓库 verify 用 `cargo run`（Debug），只学**流程**，真对比请 `cargo run --release`。

二、写法对比
1. 直观版: `let t = Instant::now(); work(); println!("{:?}", t.elapsed());` 看一眼就完。
2. 进阶版: 固定预热 + 多次采样取**中位数**，再与对照实现比较；文档注释写明 Criterion 的 `bencher.bench_function` 落点。

三、真实用途
- 发布后回归：把热点路径登记为 `benches/` 目录下的 rs，CI 里 `cargo bench` 或 nightly 对比。
- 与剖析配合（见 `02_profile`）：粗测锁定「哪段慢」，再 flamegraph / samply。
资料: https://docs.rs/criterion/latest/criterion/
      https://doc.rust-lang.org/book/ch11-00-testing.html
      https://doc.rust-lang.org/rustc/profile-guided-optimization.html
*/

use std::time::{Duration, Instant};

const LOOP_N: u32 = 200_000;

/// 直观版：累加用索引取下标（边界检查每次循环都做）
fn sum_by_index_naive(v: &[u32]) -> u64 {
    let mut acc = 0_u64;
    for i in 0..v.len() {
        acc += v[i] as u64;
    }
    acc
}

/// 进阶版：迭代器 + 溢出友好累加（语义同，常让 LLVM 更好向量化）
fn sum_by_iter(v: &[u32]) -> u64 {
    v.iter().map(|&x| x as u64).sum()
}

/// 构建固定长度向量（与基准分离，测「纯求和」）
fn make_fixture(n: u32) -> Vec<u32> {
    (0..n).map(|i| i.wrapping_mul(31) + 7).collect()
}

fn bench_one<F>(mut f: F) -> Duration
where
    F: FnMut(),
{
    let start = Instant::now();
    f();
    start.elapsed()
}

/// 预热 + 多次采样：返回中位耗时（教学版，非 Criterion 统计严谨性）
fn bench_median(samples: u32, mut f: impl FnMut()) -> Duration {
    // 预热：JIT/分支预测/缓存（Debug 下也能减少「第一次特别慢」）
    for _ in 0..3 {
        f();
    }
    let mut times: Vec<Duration> = (0..samples)
        .map(|_| {
            let d = bench_one(&mut f);
            std::hint::black_box(d);
            d
        })
        .collect();
    times.sort_by_key(|d| d.as_nanos());
    times[times.len() / 2]
}

fn compare_sum_strategies(v: &[u32]) -> (Duration, Duration) {
    let d_naive = bench_median(5, || {
        std::hint::black_box(sum_by_index_naive(v));
    });
    let d_iter = bench_median(5, || {
        std::hint::black_box(sum_by_iter(v));
    });
    (d_naive, d_iter)
}

/// 对照：两组实现结果必须一致（基准前提是正确性）
fn assert_sums_agree(v: &[u32]) {
    let a = sum_by_index_naive(v);
    let b = sum_by_iter(v);
    assert_eq!(a, b, "两种求和结果不一致，基准无意义");
}

fn print_criterion_migration_hint() {
    println!("--- 迁到 Criterion 时（项目根）---");
    println!("  cargo add criterion --dev");
    println!("  mkdir benches && 写 bench_function(\"sum_by_iter\", |b| b.iter(|| ...))");
    println!("  cargo bench");
    println!("  文档: https://docs.rs/criterion/latest/criterion/");
}

fn run_tests() {
    let v = make_fixture(10_000);
    assert_sums_agree(&v);
    assert_eq!(sum_by_iter(&v), sum_by_index_naive(&v));

    let small = make_fixture(1_000);
    let (d_slow, d_fast) = compare_sum_strategies(&small);
    // Debug 下不强行要求谁更快（优化级别低）；只要求毫秒级内完成且中位数可测
    assert!(
        d_slow.as_secs() < 2 && d_fast.as_secs() < 2,
        "粗测应在 2s 内完成，环境异常？ slow={:?} fast={:?}",
        d_slow,
        d_fast
    );

    let full = make_fixture(LOOP_N);
    assert_sums_agree(&full);
    let once = bench_one(|| {
        std::hint::black_box(sum_by_iter(&full));
    });
    assert!(
        once.as_secs() < 5,
        "单次 {:?} 全量求和不应超过 5s（Debug 粗门槛）",
        once
    );

    println!("✅ Criterion 基准心智（Instant 粗测）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 基准心智（Instant vs Criterion）************\n");

    let v = make_fixture(LOOP_N);
    println!("固定数组长度 n = {}", v.len());

    let naive = sum_by_index_naive(&v);
    let iter = sum_by_iter(&v);
    println!("结果校验: naive={} iter={}", naive, iter);

    let (d_naive, d_iter) = compare_sum_strategies(&v);
    println!(
        "中位耗时（5 次采样，Debug）:\n  sum_by_index_naive ≈ {:?}\n  sum_by_iter        ≈ {:?}",
        d_naive, d_iter
    );
    if d_iter <= d_naive {
        println!("  → 本机上迭代器版不慢于索引版（或更快）；Release 差距常更明显。");
    } else {
        println!("  → Debug 下顺序不具代表性；请用 `cargo run --release` 再比一次。");
    }

    print_criterion_migration_hint();
    println!();

    run_tests();
}

fn main() {
    bootstrap();
}