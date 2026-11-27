/*
[主题]: Option 惯用法：map / unwrap_or / if let，安全处理「可能没有」
[难度]: Easy
[前置]: 本阶段 01_enum（Option 本身是标准库枚举）

一、主题说明
Option<T> 只有两种形态：Some(有值) 或 None(没有)。
查字典、解析用户输入、数据库「按 id 找一行」都会返回 Option，而不是用 null 指针。
生产代码少用手动 match 大块重复；多用 map、and_then、unwrap_or / unwrap_or_else、if let。

二、写法对比
1. 直观版: 层层 if let / unwrap（演示时要谨慎），默认值手写分支。
2. 进阶版: map 转换、unwrap_or 给默认、if let 只处理关心的一种情况，链式更清晰。

三、真实用途
- `HashMap::get` → Option<&V>；找不到时 unwrap_or_default 或返回 404。
- 配置项可选：环境变量没有就用默认端口。
资料: https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/std/option/
      https://doc.rust-lang.org/rust-by-example/
      https://doc.rust-lang.org/rust-by-example/error/option_unwrap.html
*/

/// 直观版：用 Option 参数，手写 match 取折扣（百分比 0~100）
fn discount_percent_naive(maybe: Option<i32>) -> i32 {
    match maybe {
        Some(p) => {
            if p < 0 {
                0
            } else if p > 100 {
                100
            } else {
                p
            }
        }
        None => 0,
    }
}

/// 进阶版：map 只在 Some 时变换，否则保持 None，最后 unwrap_or
fn clamp_percent(p: i32) -> i32 {
    p.clamp(0, 100)
}

fn discount_percent_better(maybe: Option<i32>) -> i32 {
    maybe.map(clamp_percent).unwrap_or(0)
}

/// 计算折后价（分）；没有折扣当 0%
fn price_after_discount_cents(base_cents: i32, discount: Option<i32>) -> i32 {
    let pct = discount_percent_better(discount);
    let off = base_cents.saturating_mul(pct) / 100;
    base_cents.saturating_sub(off)
}

/// 直观版：根据用户 id 从「假数据库」取昵称
fn lookup_name_naive(id: u32, table: &[(&str, u32)]) -> String {
    for (name, uid) in table {
        if *uid == id {
            return name.to_string();
        }
    }
    String::from("访客")
}

/// 进阶版：find 返回 Option，再用 map / unwrap_or
fn lookup_name_better(id: u32, table: &[(&str, u32)]) -> String {
    table
        .iter()
        .find(|(_, uid)| *uid == id)
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| String::from("访客"))
}

/// 解析可选的配置端口：字符串 -> Option<u16> -> 默认值
fn parse_port_naive(raw: Option<&str>) -> u16 {
    match raw {
        Some(s) => match s.parse::<u16>() {
            Ok(p) => p,
            Err(_) => 8080,
        },
        None => 8080,
    }
}

fn parse_port_better(raw: Option<&str>) -> u16 {
    raw.and_then(|s| s.parse().ok()).unwrap_or(8080)
}

/// if let：只关心 Some 时执行一块逻辑
fn greet_if_present(maybe_name: Option<&str>) {
    if let Some(name) = maybe_name {
        println!("你好，{name}！");
    } else {
        println!("你好，访客！");
    }
}

/// 链式：先 map 再过滤 —— 用 and_then 表达「有值且满足条件才继续」
fn positive_double(maybe: Option<i32>) -> Option<i32> {
    maybe.and_then(|n| if n > 0 { Some(n * 2) } else { None })
}

fn run_tests() {
    assert_eq!(discount_percent_naive(None), 0);
    assert_eq!(discount_percent_naive(Some(15)), 15);
    assert_eq!(discount_percent_naive(Some(999)), 100);
    assert_eq!(discount_percent_better(Some(-3)), 0);
    assert_eq!(discount_percent_better(Some(20)), 20);

    assert_eq!(price_after_discount_cents(1000, Some(10)), 900);
    assert_eq!(price_after_discount_cents(500, None), 500);

    let users = [("阿 Rust", 1u32), ("小明", 2u32)];
    assert_eq!(lookup_name_naive(2, &users), "小明");
    assert_eq!(lookup_name_naive(99, &users), "访客");
    assert_eq!(lookup_name_better(1, &users), "阿 Rust");
    assert_eq!(lookup_name_better(0, &users), "访客");

    assert_eq!(parse_port_naive(None), 8080);
    assert_eq!(parse_port_naive(Some("3000")), 3000);
    assert_eq!(parse_port_naive(Some("oops")), 8080);
    assert_eq!(parse_port_better(Some("9090")), 9090);
    assert_eq!(parse_port_better(Some("bad")), 8080);

    assert_eq!(positive_double(Some(3)), Some(6));
    assert_eq!(positive_double(Some(0)), None);
    assert_eq!(positive_double(None), None);

    println!("✅ Option 惯用法 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L02 Option 惯用法 ************\n");

    println!(
        "折后价 1000 分、九折: {} 分",
        price_after_discount_cents(1000, Some(10))
    );

    let table = [("讲师", 42u32)];
    println!("用户 42: {}", lookup_name_better(42, &table));

    println!("端口: {}", parse_port_better(Some("5500")));
    greet_if_present(Some("学习者"));
    greet_if_present(None);

    if let Some(v) = positive_double(Some(7)) {
        println!("正数翻倍: {v}");
    }

    run_tests();
}

fn main() {
    bootstrap();
}