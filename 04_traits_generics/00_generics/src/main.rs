/*
[主题]: 泛型函数与泛型结构体 —— 一份代码，多种类型
[难度]: Easy → Medium
[前置]: 00_rust_basics、02_structs_enums、03_error_collections（函数与结构体已会）

一、主题说明
- **泛型**像「类型占位符」：写一次逻辑，`T` 在编译时换成具体类型（`i32`、`String`…）。
- 泛型函数：`fn foo<T>(x: T) -> T`；编译器会为每个用到的具体类型生成一份机器码（**单态化**）。
- 泛型结构体：`struct Pair<T> { left: T, right: T }`，同一套字段定义，不同类型各用各的。
- 约束入门：需要比较时用 `T: PartialOrd`；需要打印演示用 `T: std::fmt::Debug`。

二、写法对比
1. 直观版: 为 `i32` 和 `f64` 各写一份 `max_*`，结构体 `IntBox` / `StringBox` 分开定义。
2. 进阶版: `largest<T: PartialOrd>`、`Box<T>`、`Pair<T, U>`，用一处泛型消灭重复。

三、真实用途
- `Vec<T>`、`Option<T>`、`Result<T, E>` 全是泛型；库作者靠泛型写可复用 API。
- Web 里「同一种分页结构，元素有时是 DTO A 有时是 DTO B」也会见到 `Vec<T>`。
资料: https://doc.rust-lang.org/book/ch10-00-generics.html
      https://doc.rust-lang.org/book/ch10-01-syntax.html
      https://doc.rust-lang.org/rust-by-example/generics.html
      https://doc.rust-lang.org/book/
      https://www.youtube.com/@RustVideos
*/

// ── 直观版：没有泛型时只能复制粘贴 ──────────────────────────────

fn max_i32(a: i32, b: i32) -> i32 {
    if a >= b {
        a
    } else {
        b
    }
}

fn max_f64(a: f64, b: f64) -> f64 {
    if a >= b {
        a
    } else {
        b
    }
}

/// 直观版：整型专用「盒子」
struct IntBox {
    value: i32,
}

/// 直观版：字符串专用「盒子」（字段类型不同，结构不得不拆成两个）
struct StringBox {
    value: String,
}

// ── 进阶版：泛型函数与结构体 ────────────────────────────────────

/// 进阶：切片里找最大元素；空切片返回 `None`
fn largest<T: PartialOrd>(items: &[T]) -> Option<&T> {
    let first = items.first()?;
    let mut best = first;
    for item in items.iter().skip(1) {
        if item > best {
            best = item;
        }
    }
    Some(best)
}

/// 泛型结构体：一个类型参数
struct BoxValue<T> {
    value: T,
}

impl<T> BoxValue<T> {
    fn new(value: T) -> Self {
        Self { value }
    }

    fn get(&self) -> &T {
        &self.value
    }
}

/// 两个类型参数：左右可以不同类型（例如 id + 名字）
struct Pair<T, U> {
    left: T,
    right: U,
}

impl<T, U> Pair<T, U> {
    fn new(left: T, right: U) -> Self {
        Self { left, right }
    }
}

/// 混合用法：只有 `T` 需要能调试打印时，给 `T` 单独加约束即可
fn debug_pair_left<T: std::fmt::Debug, U>(p: &Pair<T, U>) -> String {
    format!("左字段 Debug = {:?}", p.left)
}

fn run_tests() {
    assert_eq!(max_i32(3, 7), 7);
    assert_eq!(max_f64(3.5, 2.1), 3.5);

    let ib = IntBox { value: 42 };
    assert_eq!(ib.value, 42);
    let sb = StringBox {
        value: "hi".to_string(),
    };
    assert_eq!(sb.value, "hi");

    assert_eq!(largest(&[1, 5, 3]), Some(&5));
    assert_eq!(largest(&[1.0_f64, 2.5, 2.0]), Some(&2.5));
    assert_eq!(largest::<i32>(&[]), None);

    let b = BoxValue::new(String::from("泛型"));
    assert_eq!(b.get(), "泛型");

    let p = Pair::new(100_u32, "订单");
    assert_eq!(p.left, 100);
    assert_eq!(p.right, "订单");
    assert!(debug_pair_left(&p).contains("100"));

    println!("✅ Generics 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Generics ************\n");

    println!("直观 max: i32={}, f64={}", max_i32(10, 4), max_f64(10.0, 4.0));

    if let Some(v) = largest(&[30, 12, 99, 7]) {
        println!("进阶 largest 在 [30,12,99,7] 中 = {v}");
    }

    let score = BoxValue::new(95_u8);
    println!("BoxValue 存分数: {}", score.get());

    let ticket = Pair::new("T-001", 2_u32);
    println!(
        "Pair 票号={} 人数={}",
        ticket.left, ticket.right
    );

    run_tests();
}

fn main() {
    bootstrap();
}