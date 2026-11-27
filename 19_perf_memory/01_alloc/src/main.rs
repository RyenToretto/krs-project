/*
[主题]: 分配直觉 —— 预分配、`push` 扩容曲线与无谓 `clone`
[难度]: Medium
[前置]: `01_ownership_borrowing`、`13_testing/00_unit`

一、主题说明
- Rust 性能里常见敌人不是「慢一点算术」，而是 **堆分配次数** 与 **临时克隆**。
- `Vec::push` 在容量不足时会按增长策略**重新分配并拷贝**整段内存；`Vec::with_capacity(n)` 可一次拿够（仍可能略大于 n）。
- `clone()` 深拷贝：在热路径上对 `String` / `Vec` 频繁 clone，CPU 与分配器压力都会上去；优先 `&T`、借用、`Cow`（后续阶段）。

二、写法对比
1. 直观版: `Vec::new()` 一路 `push`，能用就行。
2. 进阶版: 已知上限就 `with_capacity`；只读聚合用 `&[T]`，避免 `to_vec()` 再处理。

三、真实用途
- HTTP 组包前 `Vec::with_capacity(body_len)`；JSON 序列化前先 `reserve`。
- 日志里看 **分配速率**（bytes/s）与火焰图上的 `alloc::` 同看本课直觉。
资料: https://docs.rs/criterion/latest/criterion/
      https://doc.rust-lang.org/std/vec/struct.Vec.html#method.with_capacity
      https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590
*/

use std::time::Instant;

const BUILD_N: usize = 120_000;

/// 直观版：不预留，观察多次扩容（通过 `capacity()` 感受）
fn build_vec_push_only(n: usize) -> Vec<u32> {
    let mut v = Vec::new();
    for i in 0..n {
        v.push(i as u32);
    }
    v
}

/// 进阶版：一次预留，push 期间不发生因容量不足的重分配
fn build_vec_with_capacity(n: usize) -> Vec<u32> {
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        v.push(i as u32);
    }
    v
}

fn bench_build<F>(n: usize, mut f: F) -> std::time::Duration
where
    F: FnMut(usize) -> Vec<u32>,
{
    let start = Instant::now();
    let out = f(n);
    std::hint::black_box(out);
    start.elapsed()
}

/// 克隆路径：先 `to_vec` 再算（多一次分配 + 拷贝）
fn total_len_by_clone(rows: &[String]) -> usize {
    let owned = rows.to_vec();
    owned.iter().map(|s| s.len()).sum()
}

/// 借用路径：只读不改所有权
fn total_len_by_ref(rows: &[String]) -> usize {
    rows.iter().map(|s| s.len()).sum()
}

fn make_string_rows(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("row-{:06}-payload", i))
        .collect()
}

/// 估算 `push_only` 经历了多少次扩容（capacity 翻倍策略的教学计数）
fn count_growth_steps(final_cap: usize) -> usize {
    let mut cap = 0_usize;
    let mut steps = 0_usize;
    while cap < final_cap {
        if cap == 0 {
            cap = 4; // 常见首容量（实现相关，仅教学近似）
        } else {
            cap = cap.saturating_mul(2);
        }
        steps += 1;
    }
    steps
}

fn run_tests() {
    let n = BUILD_N;

    let v_push = build_vec_push_only(n);
    let v_cap = build_vec_with_capacity(n);
    assert_eq!(v_push, v_cap, "两种构建结果应一致");
    assert_eq!(v_cap.len(), n);
    assert_eq!(v_cap.capacity(), n, "with_capacity(n) 后容量应至少为 n");

    let steps = count_growth_steps(v_push.capacity());
    assert!(
        steps >= 2,
        "无预留时通常经历多次扩容（教学计数 steps={})",
        steps
    );

    let d_push = bench_build(n, build_vec_push_only);
    let d_cap = bench_build(n, build_vec_with_capacity);
    assert!(
        d_push.as_secs() < 3 && d_cap.as_secs() < 3,
        "构建应在数秒内完成 push={:?} cap={:?}",
        d_push,
        d_cap
    );
    // 预分配不应显著更慢（Debug 下多数更快）
    assert!(
        d_cap <= d_push * 2,
        "with_capacity 不应比无脑 push 慢一倍以上 cap={:?} push={:?}",
        d_cap,
        d_push
    );

    let rows = make_string_rows(2_000);
    assert_eq!(
        total_len_by_clone(&rows),
        total_len_by_ref(&rows),
        "克隆与借用聚合结果应相同"
    );
    let d_clone = bench_clone_vs_ref(&rows);
    assert!(
        d_clone.1 <= d_clone.0 * 3,
        "借用路径不应比克隆慢太多 ref={:?} clone={:?}",
        d_clone.1,
        d_clone.0
    );

    println!("✅ 分配直觉（预分配 / clone）所有测试用例通过！");
}

fn bench_clone_vs_ref(rows: &[String]) -> (std::time::Duration, std::time::Duration) {
    let start = Instant::now();
    for _ in 0..50 {
        std::hint::black_box(total_len_by_clone(rows));
    }
    let d_clone = start.elapsed();

    let start = Instant::now();
    for _ in 0..50 {
        std::hint::black_box(total_len_by_ref(rows));
    }
    let d_ref = start.elapsed();
    (d_clone, d_ref)
}

fn bootstrap() {
    println!("\n\n************ L01 分配直觉 ************\n");

    let n = BUILD_N;
    let v0 = build_vec_push_only(n);
    println!(
        "push_only: len={} capacity={}（估算扩容轮次 ≈ {}）",
        v0.len(),
        v0.capacity(),
        count_growth_steps(v0.capacity())
    );

    let v1 = build_vec_with_capacity(n);
    println!(
        "with_capacity: len={} capacity={}（push 期间无因满而 realloc）",
        v1.len(),
        v1.capacity()
    );

    let d_push = bench_build(n, build_vec_push_only);
    let d_cap = bench_build(n, build_vec_with_capacity);
    println!("\n构建 {} 个 u32（Debug 粗测）:", n);
    println!("  Vec::new + push     ≈ {:?}", d_push);
    println!("  with_capacity + push ≈ {:?}", d_cap);

    let rows = make_string_rows(1_000);
    let (dc, dr) = bench_clone_vs_ref(&rows);
    println!(
        "\n1000 行 String × 50 轮聚合:\n  先 to_vec 再算 ≈ {:?}\n  直接 & 引用遍历 ≈ {:?}",
        dc, dr
    );
    println!("正式微基准仍推荐: https://docs.rs/criterion/latest/criterion/\n");

    run_tests();
}

fn main() {
    bootstrap();
}