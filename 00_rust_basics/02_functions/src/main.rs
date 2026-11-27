/*
[主题]: 函数：参数、返回值、表达式、语句
[难度]: Easy
[前置]: 00_rust_basics/00_hello、01_control

一、主题说明
Rust 函数用 fn；最后一个表达式可不写 return；语句以分号结尾。
本课只在同一文件里用函数拆分逻辑；真正的 mod/crate 放到 05_cargo_modules。

二、写法对比
1. 直观版: 显式 return、可变局部变量逐步算。
2. 进阶版: 表达式返回、小函数组合、早期返回处理边界。

三、真实用途
- 把 handler / 校验 / 计算拆成可测纯函数。
- 金额分、折扣、限流计数等都应先写成清晰函数再接线。
资料: https://doc.rust-lang.org/book/ch03-03-how-functions-work.html
      https://doc.rust-lang.org/rust-by-example/fn.html
      https://doc.rust-lang.org/book/
*/

/// 直观版：显式 return
fn area_naive(width: i32, height: i32) -> i32 {
    let mut area = width * height;
    if area < 0 {
        area = 0;
    }
    return area;
}

/// 进阶版：表达式返回 + 边界 early return
fn area_better(width: i32, height: i32) -> i32 {
    if width <= 0 || height <= 0 {
        return 0;
    }
    width * height
}

/// 直观版：折扣后价格（分），逐步变量
fn discount_price_naive(cents: i32, percent_off: i32) -> i32 {
    let mut result = cents;
    let off = cents * percent_off / 100;
    result = result - off;
    if result < 0 {
        result = 0;
    }
    result
}

/// 进阶版：组合小函数
fn clamp_non_negative(v: i32) -> i32 {
    if v < 0 { 0 } else { v }
}

fn discount_price_better(cents: i32, percent_off: i32) -> i32 {
    let off = cents * percent_off / 100;
    clamp_non_negative(cents - off)
}

/// 多返回值用元组（入门够用；复杂结构后面用 struct）
fn div_mod(a: i32, b: i32) -> (i32, i32) {
    (a / b, a % b)
}

fn run_tests() {
    assert_eq!(area_naive(3, 4), 12);
    assert_eq!(area_naive(-1, 4), 0);
    assert_eq!(area_better(3, 4), 12);
    assert_eq!(area_better(0, 4), 0);

    assert_eq!(discount_price_naive(1000, 20), 800);
    assert_eq!(discount_price_better(1000, 20), 800);
    assert_eq!(discount_price_better(100, 150), 0);

    assert_eq!(div_mod(10, 3), (3, 1));

    println!("✅ Functions 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Functions ************\n");
    println!("面积 3x4 = {}", area_better(3, 4));
    println!("1000 分打八折 = {} 分", discount_price_better(1000, 20));
    let (q, r) = div_mod(10, 3);
    println!("10 / 3 = {} 余 {}", q, r);
    run_tests();
}

fn main() {
    bootstrap();
}
