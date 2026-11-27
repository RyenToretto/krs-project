/*
[主题]: 切片（Slice）——一段连续元素的「视图」
[难度]: Easy
[前置]: 01_ownership_borrowing/01_borrow

一、主题说明
切片是对数组 / Vec / String 某一段的借用，**不拥有**底层数据。
- &[i32]：整数切片
- &str：字符串切片（UTF-8 字节序列的视图）
创建切片不会拷贝整段数据，适合传「子区间」给函数。

二、写法对比
1. 直观版: &v[start..end]、&s[lo..hi]。
2. 进阶版: 用 split_at、strip_prefix 等返回子切片，表达解析逻辑。

三、真实用途
- 解析日志行、token、路径段：&str 切片零拷贝。
- 图像/音频 buffer、网络包 payload：&[u8]。
- API 分页取 records[off..off+limit]。
资料: https://doc.rust-lang.org/book/ch04-03-slices.html
      https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html
      https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/
*/

/// 直观版：取 Vec 的前 n 个元素（不足 n 就全取）
fn head_naive(v: &Vec<i32>, n: usize) -> &[i32] {
    let end = n.min(v.len());
    &v[0..end]
}

/// 进阶版：参数直接用切片类型，调用方可用数组或 Vec 的切片部分
fn head_better(v: &[i32], n: usize) -> &[i32] {
    let end = n.min(v.len());
    &v[0..end]
}

/// 直观版：第一个词（空格前）
fn first_word_naive(s: &str) -> &str {
    match s.find(' ') {
        Some(idx) => &s[..idx],
        None => s,
    }
}

/// 进阶版：用 split_whitespace，空串返回 ""
fn first_word_better(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

/// 找子串之前的前缀（配置键 "db.host=..." → "db.host"）
fn key_part(line: &str) -> &str {
    match line.split_once('=') {
        Some((k, _)) => k.trim(),
        None => line.trim(),
    }
}

/// 中间一段切片
fn middle_three(v: &[i32]) -> &[i32] {
    if v.len() <= 3 {
        v
    } else {
        let start = v.len() / 2 - 1;
        &v[start..start + 3]
    }
}

fn run_tests() {
    let v = vec![1, 2, 3, 4, 5];
    assert_eq!(head_naive(&v, 2), &[1, 2]);
    assert_eq!(head_better(&v, 99), &[1, 2, 3, 4, 5]);
    assert_eq!(head_better(&[7, 8], 1), &[7]);

    assert_eq!(first_word_naive("hello world"), "hello");
    assert_eq!(first_word_naive("solo"), "solo");
    assert_eq!(first_word_better("  rust  linux"), "rust");
    assert_eq!(first_word_better("   "), "");

    assert_eq!(key_part("port=8080"), "port");
    assert_eq!(key_part("name"), "name");

    assert_eq!(middle_three(&[1, 2, 3]), &[1, 2, 3]);
    assert_eq!(middle_three(&[1, 2, 3, 4, 5]), &[2, 3, 4]);

    let text = String::from("api/v1/users");
    let prefix = first_word_better(&text); // String 可自动借成 &str
    assert_eq!(prefix, "api/v1/users"); // 无空格，整串即一词

    println!("✅ Slices 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Slices ************\n");

    let nums = vec![10, 20, 30, 40];
    println!("前 2 个: {:?}", head_better(&nums, 2));

    let line = "user_id=42";
    println!("键: {}", key_part(line));
    println!("首词: {}", first_word_better("Rust 所有权"));

    run_tests();
}

fn main() {
    bootstrap();
}