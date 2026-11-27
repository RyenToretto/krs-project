/*
[主题]: Result 与 ? —— 可恢复错误的 Rust 惯用法
[难度]: Easy → Medium
[前置]: 00_rust_basics、02_structs_enums（Option 有帮助，本课从零讲 Result）

一、主题说明
- `Result<T, E>` 表示「成功带 T」或「失败带 E」；常见 `E` 入门先用 `String`。
- `Ok(x)` / `Err(e)` 与 `match` 是 foundation；`?` 在返回 `Result` 的函数里把 Err 往上冒泡。
- 业务里「用户输入错了」用 Result；「绝对不该发生」才考虑 panic（测试里 assert 可以 panic）。

二、写法对比
1. 直观版: 层层 match，每一步手动拆 Ok/Err。
2. 进阶版: `map` / `map_err` / `?`，组合多步校验。

三、真实用途
- HTTP handler：解析 JSON、校验参数 → `Result`，最终映射成 4xx/5xx。
- 读配置、连数据库：失败要说明原因，不能 silent unwrap。
资料: https://doc.rust-lang.org/book/ch09-00-error-handling.html
      https://doc.rust-lang.org/std/result/
      https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/
      https://www.youtube.com/@RustVideos
*/

/// 直观版：用 match 处理解析 + 范围
fn parse_age_naive(raw: &str) -> Result<u32, String> {
    match raw.trim().parse::<u32>() {
        Ok(n) => {
            if n <= 150 {
                Ok(n)
            } else {
                Err("年龄不能超过 150".to_string())
            }
        }
        Err(e) => Err(format!("不是合法数字: {e}")),
    }
}

/// 进阶版：先 ? 传播解析错误，再单独校验
fn parse_age_better(raw: &str) -> Result<u32, String> {
    let n = raw
        .trim()
        .parse::<u32>()
        .map_err(|e| format!("不是合法数字: {e}"))?;
    if n > 150 {
        return Err("年龄不能超过 150".to_string());
    }
    Ok(n)
}

/// 直观版：注册流程，两步都手写 match
fn register_naive(name: &str, age_raw: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("名字不能为空".to_string());
    }
    match parse_age_naive(age_raw) {
        Ok(age) => Ok(format!("欢迎 {name}，年龄 {age}")),
        Err(e) => Err(e),
    }
}

/// 进阶版：? 串联；错误留在同一层返回
fn register_better(name: &str, age_raw: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("名字不能为空".to_string());
    }
    let age = parse_age_better(age_raw)?;
    Ok(format!("欢迎 {name}，年龄 {age}"))
}

/// 演示 ?：任何 Err 会立刻从当前函数返回
fn load_profile_step(name: &str, age_raw: &str) -> Result<String, String> {
    let greeting = register_better(name, age_raw)?;
    Ok(format!("档案已加载: {greeting}"))
}

/// 把 Result 转成「有默认值」—— 直观 match
fn age_or_default_naive(raw: &str, default: u32) -> u32 {
    match parse_age_better(raw) {
        Ok(n) => n,
        Err(_) => default,
    }
}

/// 进阶：unwrap_or（仅当失败可接受时用）
fn age_or_default_better(raw: &str, default: u32) -> u32 {
    parse_age_better(raw).unwrap_or(default)
}

fn run_tests() {
    assert_eq!(parse_age_naive("25"), Ok(25));
    assert_eq!(parse_age_better("25"), Ok(25));
    assert!(parse_age_naive("abc").is_err());
    assert!(parse_age_better("200").is_err());

    assert_eq!(
        register_naive("Dave", "30").unwrap(),
        "欢迎 Dave，年龄 30"
    );
    assert_eq!(
        register_better("Alice", "30").unwrap(),
        "欢迎 Alice，年龄 30"
    );
    assert!(register_better("", "30").is_err());
    assert_eq!(
        load_profile_step("Bob", "40").unwrap(),
        "档案已加载: 欢迎 Bob，年龄 40"
    );

    assert_eq!(age_or_default_naive("bad", 18), 18);
    assert_eq!(age_or_default_better("22", 18), 22);

    println!("✅ Result / ? 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Result / ? ************\n");

    match register_better("小陈", "28") {
        Ok(msg) => println!("✔ {msg}"),
        Err(e) => println!("✗ {e}"),
    }

    match load_profile_step("小陈", "not-a-number") {
        Ok(msg) => println!("✔ {msg}"),
        Err(e) => println!("✗ 加载失败（预期演示）: {e}"),
    }

    println!(
        "缺省年龄演示: {}",
        age_or_default_better("oops", 20)
    );

    run_tests();
}

fn main() {
    bootstrap();
}