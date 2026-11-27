/*
[主题]: 单元测试 —— 表驱动（数组用例）与简单基准计时
[难度]: Easy
[前置]: `00_rust_basics`（assert_eq!、`Result`）、`04_traits_generics`（可选）

一、主题说明
- **单元测试**只验证一小段纯逻辑（解析、校验、计算），输入输出可预测、不碰真 DB/网络。
- Rust 常见两种写法：`#[cfg(test)] mod tests` + `cargo test`（本仓库 verify 用 `cargo run` 里的 `run_tests`）；表驱动则把多组 **(输入, 期望)** 放进数组循环断言。
- **表驱动**好处：新增用例只加一行 struct，失败时打印 `name` 一眼定位；团队协作时 PR 里用例表就是规格说明。

二、写法对比
1. 直观版: 每个场景单独 `assert_eq!(parse_cents("1.00"), Ok(100))`，复制粘贴十几次。
2. 进阶版: `struct Case { name, input, want }` + `for case in CASES`，统一 `assert_eq!(got, case.want, "{}", case.name)`。

三、真实用途
- Axum handler 前的 **DTO 校验**、金额/税率、权限字符串解析 —— 都先表驱动单元测，再写集成测。
- 性能回归：生产用 [Criterion](https://docs.rs/criterion/latest/criterion/)；入门用 `Instant::now()` 粗测即可。
资料: https://doc.rust-lang.org/book/ch11-00-testing.html
      https://doc.rust-lang.org/rust-by-example/testing.html
      https://docs.rs/criterion/latest/criterion/
*/

use std::time::Instant;

/// 把「元.分」字符串解析为分（cents），非法输入返回 Err。
fn parse_cents_naive(raw: &str) -> Result<i64, &'static str> {
    let parts: Vec<&str> = raw.split('.').collect();
    if parts.len() != 2 {
        return Err("需要恰好一个小数点");
    }
    let yuan: i64 = parts[0]
        .parse()
        .map_err(|_| "整数部分不是数字")?;
    let frac = parts[1];
    if frac.len() != 2 {
        return Err("小数部分必须是两位");
    }
    let fen: i64 = frac.parse().map_err(|_| "小数部分不是数字")?;
    if yuan < 0 || fen < 0 {
        return Err("不允许负数");
    }
    Ok(yuan * 100 + fen)
}

/// 进阶版：先 trim、拒绝空串，错误文案更统一（便于表驱动断言 Err 变体）。
fn parse_cents(raw: &str) -> Result<i64, &'static str> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("空输入");
    }
    parse_cents_naive(raw)
}

#[derive(Debug, Clone, Copy)]
struct CentsCase {
    name: &'static str,
    input: &'static str,
    want: Result<i64, &'static str>,
}

/// 两套实现行为一致的用例（表驱动核心）
const PARSE_SHARED: &[CentsCase] = &[
    CentsCase {
        name: "正常两位小数",
        input: "12.34",
        want: Ok(1234),
    },
    CentsCase {
        name: "整数部分是 0",
        input: "0.01",
        want: Ok(1),
    },
    CentsCase {
        name: "缺少小数点",
        input: "100",
        want: Err("需要恰好一个小数点"),
    },
    CentsCase {
        name: "小数位不是两位",
        input: "1.2",
        want: Err("小数部分必须是两位"),
    },
    CentsCase {
        name: "非数字整数部",
        input: "x.00",
        want: Err("整数部分不是数字"),
    },
];

/// 仅进阶版 `parse_cents`（trim / 空串）
const PARSE_ADVANCED_ONLY: &[CentsCase] = &[
    CentsCase {
        name: "带首尾空白",
        input: "  9.99  ",
        want: Ok(999),
    },
    CentsCase {
        name: "空串",
        input: "",
        want: Err("空输入"),
    },
];

fn run_table_driven(cases: &[CentsCase], f: fn(&str) -> Result<i64, &'static str>, label: &str) {
    for case in cases {
        let got = f(case.input);
        assert_eq!(
            got, case.want,
            "[{}] 用例「{}」input={:?}",
            label, case.name, case.input
        );
    }
}

/// 不用 Criterion：对同一函数跑 N 次，打印总耗时（仅教学，不作 CI 门槛）。
fn bench_parse_loops(n: u32) -> std::time::Duration {
    let start = Instant::now();
    let mut acc = 0_i64;
    for i in 0..n {
        let s = format!("{}.{}", i % 1000, i % 100);
        acc += parse_cents(&s).unwrap_or(0);
    }
    let _ = acc;
    start.elapsed()
}

fn run_tests() {
    // 直观版：手写两条（体会重复）
    assert_eq!(parse_cents_naive("1.00").unwrap(), 100);
    assert_eq!(parse_cents_naive("2.50").unwrap(), 250);

    // 表驱动：同一套 CASES 可测 naive 与进阶（进阶多处理 trim/空串）
    run_table_driven(PARSE_SHARED, parse_cents_naive, "naive");
    run_table_driven(PARSE_SHARED, parse_cents, "parse_cents");
    run_table_driven(PARSE_ADVANCED_ONLY, parse_cents, "parse_cents+");

    let dur = bench_parse_loops(50_000);
    assert!(dur.as_secs() < 5, "粗基准不应超过 5s（环境异常？）");

    println!("✅ 表驱动单元测试与粗计时所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 表驱动单元测试 ************\n");

    println!("直观版: 1.00 → {} 分", parse_cents_naive("1.00").unwrap());
    println!("进阶版: \"  3.14  \" → {} 分", parse_cents("  3.14  ").unwrap());

    let total = PARSE_SHARED.len() + PARSE_ADVANCED_ONLY.len();
    println!("\n表驱动用例数: {}", total);
    for case in PARSE_SHARED.iter().take(3) {
        println!("  - {} → {:?}", case.name, parse_cents(case.input));
    }

    let dur = bench_parse_loops(100_000);
    println!(
        "\n粗基准（非 Criterion）: 10 万次 parse_cents ≈ {:?}",
        dur
    );
    println!("正式微基准请见: https://docs.rs/criterion/latest/criterion/\n");

    run_tests();
}

fn main() {
    bootstrap();
}