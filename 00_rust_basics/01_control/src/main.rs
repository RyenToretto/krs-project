/*
[主题]: 控制流：if / match / for / loop / while
[难度]: Easy
[前置]: 00_rust_basics/00_hello

一、主题说明
Rust 的条件与循环：if 是表达式；match 必须穷尽；loop 可带返回值；
for 常遍历迭代器；while 适合条件循环。本课按小白从「像其他语言」过渡到「Rust 惯用」。

二、写法对比
1. 直观版: if/else、while、C 风格 for 范围。
2. 进阶版: match 表达式、loop+break 返回值、迭代器 for。

三、真实用途
- HTTP 状态码分支、重试循环、批处理遍历。
- 后端里「按枚举状态机推进」几乎都用 match。
资料: https://doc.rust-lang.org/book/ch03-05-control-flow.html
      https://doc.rust-lang.org/rust-by-example/flow_control.html
      https://doc.rust-lang.org/book/
*/

/// 直观版：用 if/else 把分数映射成等级
fn grade_naive(score: i32) -> &'static str {
    // if 是表达式，但这里先用语句风格，方便和小白其他语言对照
    if score >= 90 {
        "A"
    } else if score >= 80 {
        "B"
    } else if score >= 60 {
        "C"
    } else {
        "D"
    }
}

/// 进阶版：match 穷尽分支，更适合「离散档位」
fn grade_better(score: i32) -> &'static str {
    match score {
        90..=100 => "A",
        80..=89 => "B",
        60..=79 => "C",
        0..=59 => "D",
        // 超出合理区间时仍给出明确结果，避免 panic（真实系统常返回 Err）
        _ => "?",
    }
}

/// 直观版：while 累加 1..=n
fn sum_while_naive(n: i32) -> i32 {
    let mut i = 1;
    let mut total = 0;
    while i <= n {
        total += i;
        i += 1;
    }
    total
}

/// 进阶版：for 遍历范围；或用公式（这里用 for 练手）
fn sum_for_better(n: i32) -> i32 {
    let mut total = 0;
    for i in 1..=n {
        total += i;
    }
    total
}

/// 进阶版：loop + break 带返回值（适合「重试直到成功」的骨架）
fn first_even_from(start: i32) -> i32 {
    let mut x = start;
    loop {
        if x % 2 == 0 {
            break x;
        }
        x += 1;
    }
}

fn run_tests() {
    assert_eq!(grade_naive(95), "A");
    assert_eq!(grade_naive(85), "B");
    assert_eq!(grade_naive(70), "C");
    assert_eq!(grade_naive(50), "D");

    assert_eq!(grade_better(95), "A");
    assert_eq!(grade_better(85), "B");
    assert_eq!(grade_better(70), "C");
    assert_eq!(grade_better(50), "D");
    assert_eq!(grade_better(101), "?");

    assert_eq!(sum_while_naive(5), 15);
    assert_eq!(sum_for_better(5), 15);
    assert_eq!(first_even_from(3), 4);
    assert_eq!(first_even_from(8), 8);

    println!("✅ ControlFlow 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 ControlFlow ************\n");
    println!("95 分等级(直观): {}", grade_naive(95));
    println!("95 分等级(match): {}", grade_better(95));
    println!("1..=5 求和: {}", sum_for_better(5));
    println!("从 3 找偶数: {}", first_even_from(3));
    run_tests();
}

fn main() {
    bootstrap();
}
