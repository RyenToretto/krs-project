# `krs-project` 学习文件标准模板

## 1. 单课 `Cargo.toml` + `src/main.rs` 模板

`Cargo.toml`：

```toml
[package]
name = "l00_topic_name"
version = "0.1.0"
edition = "2021"

[dependencies]
```

`src/main.rs`：

```rust
/*
[主题]: 一句话标题
[难度]: Easy / Medium / Hard
[前置]: 无 / 00_rust_basics/...

一、主题说明
...

二、写法对比
1. 直观版: ...
2. 进阶版: ...

三、真实用途
- ...
资料: https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/
*/

fn add_naive(left: i32, right: i32) -> i32 {
    left + right
}

fn add_better(left: i32, right: i32) -> i32 {
    left + right
}

fn run_tests() {
    assert_eq!(add_naive(1, 2), 3, "失败");
    assert_eq!(add_better(1, 2), 3, "失败");
    println!("✅ Topic 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ Topic ************\n");
    run_tests();
}

fn main() {
    bootstrap();
}
```

## 2. 阶段 `README.md` 模板

必须含：目标/前置/环境、推荐资料明文 URL、章节进度表、本阶段知识点资料包、总课数。

## 3. 命名

- 目录：`00_hello/`、`01_control/`…
- 每课：`Cargo.toml` + `src/main.rs`（包名 `l00_...`）
