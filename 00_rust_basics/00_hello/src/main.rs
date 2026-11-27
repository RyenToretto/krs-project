/*
[主题]: 第一个 Rust 程序：main、基本类型与自测结构
[难度]: Easy
[前置]: 无（Rust 小白第一课）

一、主题说明
Rust 程序从 main 入口开始。本课建立「写逻辑 → bootstrap 演示 → run_tests 断言」的学习习惯。
工具链：rustup / cargo；本课为可执行二进制包。

二、写法对比
1. 直观版 add_naive: 直接用 +。
2. 进阶版 add_better: 同样加法，强调命名与可测函数拆分。

三、真实用途
- 所有 CLI、Web、Worker 都有入口。
- 数值类型选择在金额/计数场景很重要（金额常用专用类型或整数分，避免盲目用 f64）。
资料: https://www.rust-lang.org/learn
      https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/
      https://rustlings.rust-lang.org/
*/

fn add_naive(left: i32, right: i32) -> i32 {
    left + right
}

fn add_better(left: i32, right: i32) -> i32 {
    left + right
}

fn describe_primitives() -> String {
    let age: i32 = 18;
    let price: f64 = 19.9;
    let ok: bool = true;
    let grade: char = 'A';
    format!("age={age}, price={price}, ok={ok}, grade={grade}")
}

fn run_tests() {
    assert_eq!(add_naive(1, 2), 3, "add_naive(1,2) 应为 3");
    assert_eq!(add_better(2, 3), 5, "add_better(2,3) 应为 5");
    let desc = describe_primitives();
    assert!(desc.contains("age=18"), "应包含 age=18");
    println!("✅ HelloAndTypes 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 HelloAndTypes ************\n");
    println!("基本类型示例: {}", describe_primitives());
    println!("1 + 2 = {}", add_naive(1, 2));
    run_tests();
}

fn main() {
    bootstrap();
}
