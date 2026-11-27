/*
[主题]: 结构体与方法（impl）：把业务数据和行为绑在一起
[难度]: Easy
[前置]: 00_rust_basics、01_ownership_borrowing（建议已学）

一、主题说明
结构体（struct）用于命名一组相关字段，比「一堆散落的变量」更清晰。
impl 块可以写「方法」（第一个参数是 &self / &mut self）和「关联函数」（无 self，常作构造器）。
元组结构体适合只有一两个字段的简单包装；命名字段适合订单、用户等模型。

二、写法对比
1. 直观版: 手写函数，把 struct 当参数传入（像 C 里的 struct + 独立函数）。
2. 进阶版: impl 里写方法，用 &self 表达「属于这个对象的行为」，代码更贴近业务语言。

三、真实用途
- HTTP API 里用 struct 表示请求体 / 响应 DTO（Data Transfer Object）。
- impl 里放校验、算总价、格式化展示，便于单测 run_tests 覆盖。
资料: https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/book/ch05-00-structs.html
      https://doc.rust-lang.org/rust-by-example/
      https://doc.rust-lang.org/rust-by-example/custom_types/structs.html
*/

/// 命名字段：购物车一行（商品名 + 单价分 + 数量）
#[derive(Debug, PartialEq, Clone, Copy)]
struct LineItem {
    name: &'static str,
    unit_cents: i32,
    qty: i32,
}

/// 直观版：不挂在 impl 上，单独函数算小计
fn line_subtotal_naive(item: LineItem) -> i32 {
    let price = item.unit_cents * item.qty;
    if price < 0 {
        return 0;
    }
    price
}

/// 进阶版：行为写在 impl 里，读起来像「这一行自己会算小计」
impl LineItem {
    /// 关联函数：构造器（没有 self，用 LineItem::new 调用）
    fn new(name: &'static str, unit_cents: i32, qty: i32) -> Self {
        LineItem {
            name,
            unit_cents,
            qty,
        }
    }

    /// 方法：&self 表示借用自身，不拿走所有权
    fn subtotal_cents(&self) -> i32 {
        let raw = self.unit_cents.saturating_mul(self.qty);
        raw.max(0)
    }

    fn describe(&self) -> String {
        format!(
            "{} x{} @ {}分/件 = {}分",
            self.name,
            self.qty,
            self.unit_cents,
            self.subtotal_cents()
        )
    }
}

/// 元组结构体：只包一个整数，表示「已校验的非负分」
#[derive(Debug, PartialEq, Clone, Copy)]
struct Cents(i32);

impl Cents {
    fn new(value: i32) -> Self {
        Cents(value.max(0))
    }

    fn inner(self) -> i32 {
        self.0
    }
}

/// 直观版：订单总价——可变数组 + 循环累加
fn order_total_naive(items: &[LineItem]) -> i32 {
    let mut sum = 0;
    for item in items {
        sum = sum + line_subtotal_naive(*item);
    }
    sum
}

/// 进阶版：订单 struct + impl 汇总
struct Order {
    lines: Vec<LineItem>,
}

impl Order {
    fn from_lines(lines: Vec<LineItem>) -> Self {
        Order { lines }
    }

    fn total_cents(&self) -> Cents {
        let mut sum = 0i32;
        for line in &self.lines {
            sum = sum.saturating_add(line.subtotal_cents());
        }
        Cents::new(sum)
    }
}

fn run_tests() {
    let apple = LineItem::new("苹果", 350, 2);
    assert_eq!(line_subtotal_naive(apple), 700);
    assert_eq!(apple.subtotal_cents(), 700);

    let bad_qty = LineItem::new("测试", 100, -1);
    // saturating_mul 让负数数量不会溢出，再 max(0) 保底
    assert_eq!(bad_qty.subtotal_cents(), 0);

    assert_eq!(Cents::new(-5).inner(), 0);
    assert_eq!(Cents::new(99).inner(), 99);

    let items = [
        LineItem::new("牛奶", 500, 1),
        LineItem::new("面包", 800, 2),
    ];
    assert_eq!(order_total_naive(&items), 2100);

    let order = Order::from_lines(items.to_vec());
    assert_eq!(order.total_cents().inner(), 2100);

    println!("✅ Structs 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Structs ************\n");

    let line = LineItem::new("草莓", 1200, 3);
    println!("直观版小计: {} 分", line_subtotal_naive(line));
    println!("进阶版描述: {}", line.describe());

    let order = Order::from_lines(vec![
        LineItem::new("咖啡", 2800, 1),
        LineItem::new("曲奇", 450, 4),
    ]);
    println!("订单合计: {} 分", order.total_cents().inner());

    run_tests();
}

fn main() {
    bootstrap();
}