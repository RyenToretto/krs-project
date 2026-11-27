/*
[主题]: String 进阶 —— 与 &str、容量、UTF-8 校验
[难度]: Medium
[前置]: 00_rust_basics/03_strings；所有权基础

一、主题说明
- `String`：堆上、可变、拥有 UTF-8 字节；`&str`：通常是借用的一段 UTF-8 视图。
- `len()` 是字节数不是「字符个数」；遍历 Unicode 标量用 `.chars()`。
- `with_capacity` / `reserve` 减少重复分配；`String::from_utf8` 把字节变 String 时要校验 UTF-8。

二、写法对比
1. 直观版: `push_str` 拼日志；`from_utf8` 后 match 处理 Err。
2. 进阶版: 预留容量；`from_utf8` 配合 `map_err` 转成业务 `Result`。

三、真实用途
- 拼 JSON / 日志行前先 `reserve`；读网络缓冲用 `from_utf8` 区分「非法编码」与「逻辑错误」。
- API 层常接收 `&str` 借用，内部再 `to_string()` 存入 struct。
资料: https://doc.rust-lang.org/book/ch08-02-strings.html
      https://doc.rust-lang.org/std/string/struct.String.html
      https://doc.rust-lang.org/std/str/
      https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/std/str.html
      https://www.youtube.com/@RustVideos
*/

/// 直观版：反复 push_str，可能多次扩容
fn build_greeting_naive(name: &str) -> String {
    let mut s = String::new();
    s.push_str("Hello, ");
    s.push_str(name);
    s.push('!');
    s
}

/// 进阶版：按估计长度预留，减少分配
fn build_greeting_better(name: &str) -> String {
    let mut s = String::with_capacity(7 + name.len() + 1);
    s.push_str("Hello, ");
    s.push_str(name);
    s.push('!');
    s
}

/// 字节数 vs 字符数（含中文）
fn explain_lengths(s: &str) -> (usize, usize) {
    (s.len(), s.chars().count())
}

/// 直观版：把 Vec<u8> 当 UTF-8 文本读入
fn bytes_to_string_naive(bytes: Vec<u8>) -> Result<String, String> {
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(e) => Err(format!("不是合法 UTF-8: {}", e)),
    }
}

/// 进阶版：map_err 写得更短
fn bytes_to_string_better(bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|e| format!("不是合法 UTF-8: {}", e))
}

/// 直观版：清洗用户输入（trim + 合并空白）
fn normalize_naive(input: &str) -> String {
    let trimmed = input.trim();
    let mut out = String::new();
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

/// 进阶版：split_whitespace 再 join（只适用于 ASCII 空白场景入门）
fn normalize_better(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 演示 reserve：日志前缀已知长度
fn append_line_better(mut buf: String, line: &str) -> String {
    buf.reserve(line.len() + 1);
    if !buf.is_empty() {
        buf.push('\n');
    }
    buf.push_str(line);
    buf
}

fn run_tests() {
    assert_eq!(build_greeting_naive("Rust"), "Hello, Rust!");
    assert_eq!(build_greeting_better("Rust"), "Hello, Rust!");

    let (bytes, chars) = explain_lengths("Hi 你好");
    assert!(bytes > chars);
    assert_eq!(chars, 5); // Hi + 空格 + 你好（两个 Unicode 标量）

    let ok = bytes_to_string_naive(b"ok".to_vec()).unwrap();
    assert_eq!(bytes_to_string_better(b"ok".to_vec()).unwrap(), ok);
    let bad = bytes_to_string_better(vec![0xff, 0xfe, 0xfd]);
    assert!(bad.is_err());

    assert_eq!(normalize_naive("  a   b  "), "a b");
    assert_eq!(normalize_better("  a   b  "), "a b");

    let merged = append_line_better("line1".to_string(), "line2");
    assert_eq!(merged, "line1\nline2");

    println!("✅ String 进阶 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 String 进阶 ************\n");

    let name = "世界";
    println!("问候: {}", build_greeting_better(name));
    let (b, c) = explain_lengths(name);
    println!("「{name}」字节数={b}，字符数={c}");

    match bytes_to_string_better("你好".as_bytes().to_vec()) {
        Ok(s) => println!("UTF-8 解码: {s}"),
        Err(e) => println!("{e}"),
    }

    println!("规范化: «{}»", normalize_better("  foo   bar "));

    run_tests();
}

fn main() {
    bootstrap();
}