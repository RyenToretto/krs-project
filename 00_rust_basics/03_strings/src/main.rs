/*
[主题]: 字符串入门：String 与 &str
[难度]: Easy
[前置]: 00_rust_basics/00_hello、02_functions

一、主题说明
Rust 有两种常见「字符串」：
- &str：字符串切片（常指向字面量或某处内容的视图）
- String：堆上可增长的拥有型字符串
本课只建立直觉；所有权细节在 01_ownership_borrowing 展开。

二、写法对比
1. 直观版: 字面量、to_string、format!、push_str。
2. 进阶版: 接受 &str 参数（更通用）、返回 String、简单解析。

三、真实用途
- API 路径、配置键、日志消息、JSON 字段名。
- 函数参数优先 &str，需要拥有所有权时再返回/存 String。
资料: https://doc.rust-lang.org/book/ch08-02-strings.html
      https://doc.rust-lang.org/rust-by-example/std/str.html
      https://doc.rust-lang.org/std/primitive.str.html
      https://doc.rust-lang.org/std/string/struct.String.html
*/

/// 直观版：拼接问候语（全程 String）
fn greet_naive(name: String) -> String {
    let mut s = String::from("你好, ");
    s.push_str(&name);
    s.push('!');
    s
}

/// 进阶版：参数用 &str（字面量和 String 都能传）
fn greet_better(name: &str) -> String {
    format!("你好, {name}!")
}

/// 直观版：判断是否空串
fn is_blank_naive(s: &str) -> bool {
    s.len() == 0
}

/// 进阶版：trim 后再判断（真实表单常见）
fn is_blank_better(s: &str) -> bool {
    s.trim().is_empty()
}

/// 简单解析：取第一个冒号前的键（失败返回空）
fn key_before_colon(line: &str) -> &str {
    match line.split_once(':') {
        Some((key, _)) => key.trim(),
        None => "",
    }
}

fn run_tests() {
    assert_eq!(greet_naive(String::from("小白")), "你好, 小白!");
    assert_eq!(greet_better("小白"), "你好, 小白!");
    assert_eq!(greet_better(&String::from("Rust")), "你好, Rust!");

    assert!(is_blank_naive(""));
    assert!(!is_blank_naive(" "));
    assert!(is_blank_better("  \t"));
    assert!(!is_blank_better("hi"));

    assert_eq!(key_before_colon("host: localhost"), "host");
    assert_eq!(key_before_colon("no-colon"), "");

    println!("✅ StringsIntro 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 StringsIntro ************\n");
    println!("{}", greet_better("学习者"));
    println!("键: {}", key_before_colon("port: 8080"));
    run_tests();
}

fn main() {
    bootstrap();
}
