/*
[主题]: Vec 与 HashMap —— 最常用的两类集合
[难度]: Easy → Medium
[前置]: 00_rust_basics；本课同时用 Result 做「查不到」演示

一、主题说明
- `Vec<T>`：有序、可增长数组；`push` / `pop` / 下标 / 迭代。
- `HashMap<K, V>`：键值表；`insert` / `get`；`entry` API 避免「先 get 再 insert」重复查找。
- 两者都放在堆上，变量本身常是「拥有 Vec / HashMap」的句柄（和所有权课衔接）。

二、写法对比
1. 直观版: `get` + `match`，没有就新建 Vec 再 `insert`。
2. 进阶版: `entry(key).or_insert_with(...)` 一行搞定分组。

三、真实用途
- Vec：请求日志行、批量 ID、分页列表。
- HashMap：按 user_id 统计、缓存键值、配置项索引。
资料: https://doc.rust-lang.org/book/ch08-00-common-collections.html
      https://doc.rust-lang.org/std/vec/struct.Vec.html
      https://doc.rust-lang.org/std/collections/struct.HashMap.html
      https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;

/// 直观版：逐个 push
fn build_scores_naive() -> Vec<i32> {
    let mut v = Vec::new();
    v.push(90);
    v.push(85);
    v.push(88);
    v
}

/// 进阶版：宏初始化 + 再追加
fn build_scores_better() -> Vec<i32> {
    let mut v = vec![90, 85, 88];
    v.push(92);
    v
}

/// 直观版：累加 Vec
fn sum_naive(nums: &[i32]) -> i32 {
    let mut total = 0;
    for n in nums {
        total += n;
    }
    total
}

/// 进阶版：迭代器 fold（惯用）
fn sum_better(nums: &[i32]) -> i32 {
    nums.iter().copied().fold(0, |acc, x| acc + x)
}

/// 直观版：单词计数 —— 每次 get 再决定 insert 或 +1
fn word_count_naive(text: &str) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for word in text.split_whitespace() {
        let key = word.to_lowercase();
        match map.get(&key) {
            Some(count) => {
                map.insert(key, count + 1);
            }
            None => {
                map.insert(key, 1);
            }
        }
    }
    map
}

/// 进阶版：entry + or_insert
fn word_count_better(text: &str) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for word in text.split_whitespace() {
        let key = word.to_lowercase();
        *map.entry(key).or_insert(0) += 1;
    }
    map
}

/// 用 Result 表达「购物车里没有这个 sku」
fn take_one_naive(cart: &mut HashMap<String, u32>, sku: &str) -> Result<u32, String> {
    match cart.get_mut(sku) {
        Some(qty) if *qty > 0 => {
            *qty -= 1;
            Ok(*qty)
        }
        Some(_) => Err(format!("sku {sku} 库存已为 0")),
        None => Err(format!("未找到 sku: {sku}")),
    }
}

fn take_one_better(cart: &mut HashMap<String, u32>, sku: &str) -> Result<u32, String> {
    let qty = cart.get_mut(sku).ok_or_else(|| format!("未找到 sku: {sku}"))?;
    if *qty == 0 {
        return Err(format!("sku {sku} 库存已为 0"));
    }
    *qty -= 1;
    Ok(*qty)
}

fn run_tests() {
    let naive_scores = build_scores_naive();
    assert_eq!(naive_scores.len(), 3);
    let scores = build_scores_better();
    assert_eq!(scores.len(), 4);
    assert_eq!(sum_naive(&scores), 90 + 85 + 88 + 92);
    assert_eq!(sum_better(&scores), sum_naive(&scores));

    let text = "Rust rust HASH map";
    let m1 = word_count_naive(text);
    let m2 = word_count_better(text);
    assert_eq!(m1.get("rust"), Some(&2));
    assert_eq!(m2.get("hash"), Some(&1));
    assert_eq!(m1, m2);

    let mut cart: HashMap<String, u32> = HashMap::new();
    cart.insert("A001".to_string(), 2);
    assert_eq!(take_one_naive(&mut cart, "A001").unwrap(), 1);
    cart.insert("A001".to_string(), 2);
    assert_eq!(take_one_better(&mut cart, "A001").unwrap(), 1);
    assert!(take_one_better(&mut cart, "NOPE").is_err());

    println!("✅ Vec / HashMap 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Vec / HashMap ************\n");

    let scores = build_scores_better();
    println!("分数列表 {:?}，总和 {}", scores, sum_better(&scores));

    let counts = word_count_better("Tokio async Async");
    println!("词频示例: {:?}", counts);

    let mut cart = HashMap::from([("BOOK-1".to_string(), 3u32)]);
    match take_one_better(&mut cart, "BOOK-1") {
        Ok(left) => println!("扣减后剩余 {left} 本"),
        Err(e) => println!("{e}"),
    }

    run_tests();
}

fn main() {
    bootstrap();
}