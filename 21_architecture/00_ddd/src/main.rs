/*
[主题]: DDD 战术模式 —— 实体、值对象、聚合根与不变量
[难度]: Medium
[前置]: `02_structs_enums`、`04_traits_generics`、`16_production_rust`（错误心智）建议

一、主题说明
- **实体（Entity）**：靠 **标识（ID）** 区分，哪怕属性相同也不是同一个业务对象（例：两张「同金额订单」只要 ID 不同就是两笔交易）。
- **值对象（Value Object）**：靠 **值** 相等，无独立 ID；应 **不可变**、可替换（例：金额、邮箱、地址）。
- **聚合（Aggregate）**：一组对象对外只通过 **聚合根（Aggregate Root）** 修改；根内维护 **不变量**（例：订单行不能空、小计不能为负）。
- Rust 映射：实体用带 `id` 的 struct + 行为方法；值对象用私有字段 + `PartialEq`；聚合根封装 `Vec<Line>` 并拒绝非法状态。

二、写法对比
1. 直观版: `OrderDraft` 用公开字段拼装，适合理解概念，不适合直接上生产。
2. 进阶版: `Order` 聚合根私有化 `lines`，`Money` 值对象封装分（cents）与比较，非法操作返回 `Result`。

三、真实用途
- 后端限界上下文内建模；与 sqlx/仓储层解耦：聚合在领域层，持久化在基础设施层。
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) 鼓励清晰的类型边界，与值对象「小类型」一脉相承。
资料: https://rust-lang.github.io/api-guidelines/
      https://nostarch.com/rust-rustaceans
      https://www.amazon.com/s?k=Rust+for+Rustaceans
*/

use std::fmt;

// ---------------------------------------------------------------------------
// 值对象：Money（分整数，避免浮点）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Money {
    cents: u64,
}

impl Money {
    fn from_cents(cents: u64) -> Result<Self, &'static str> {
        if cents > 10_000_000_00 {
            return Err("金额超出教学上限");
        }
        Ok(Self { cents })
    }

    fn cents(self) -> u64 {
        self.cents
    }

    fn checked_add(self, other: Self) -> Result<Self, &'static str> {
        let sum = self
            .cents
            .checked_add(other.cents)
            .ok_or("金额溢出")?;
        Self::from_cents(sum)
    }

    fn checked_mul_qty(self, qty: u32) -> Result<Self, &'static str> {
        let product = (self.cents as u128)
            .checked_mul(qty as u128)
            .ok_or("乘法溢出")?;
        if product > u64::MAX as u128 {
            return Err("金额溢出");
        }
        Self::from_cents(product as u64)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.cents as f64 / 100.0)
    }
}

// ---------------------------------------------------------------------------
// 值对象：Sku（商品编号，按值比较）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Sku(String);

impl Sku {
    fn new(raw: &str) -> Result<Self, &'static str> {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.len() > 32 {
            return Err("SKU 非法");
        }
        Ok(Self(trimmed.to_ascii_uppercase()))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// 实体：OrderLine（在聚合内有行号，但同一 SKU 可重复 —— 靠行身份区分用索引）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct OrderLine {
    sku: Sku,
    unit_price: Money,
    quantity: u32,
}

impl OrderLine {
    fn line_total(&self) -> Result<Money, &'static str> {
        if self.quantity == 0 {
            return Err("数量不能为 0");
        }
        self.unit_price.checked_mul_qty(self.quantity)
    }
}

// ---------------------------------------------------------------------------
// 实体 + 聚合根：Order（根持有 OrderId，外人不能直接改 lines）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OrderId(u64);

#[derive(Debug)]
struct Order {
    id: OrderId,
    lines: Vec<OrderLine>,
}

/// 直观版：一次性数据袋，演示「能跑通 happy path」，无不变量护栏
#[derive(Debug, Default)]
struct OrderDraft {
    pub lines: Vec<(String, u64, u32)>,
}

impl OrderDraft {
    fn naive_total_cents(&self) -> u64 {
        self.lines.iter().map(|(_, price, qty)| price * (*qty as u64)).sum()
    }
}

impl Order {
    fn new(id: OrderId) -> Self {
        Self {
            id,
            lines: Vec::new(),
        }
    }

    fn id(&self) -> OrderId {
        self.id
    }

    fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// 聚合根入口：加行时统一校验
    fn add_line(&mut self, sku: Sku, unit_price: Money, quantity: u32) -> Result<(), &'static str> {
        const MAX_LINES: usize = 50;
        if self.lines.len() >= MAX_LINES {
            return Err("订单行数已达上限");
        }
        if quantity == 0 || quantity > 9_999 {
            return Err("数量非法");
        }
        self.lines.push(OrderLine {
            sku,
            unit_price,
            quantity,
        });
        Ok(())
    }

    fn subtotal(&self) -> Result<Money, &'static str> {
        if self.lines.is_empty() {
            return Err("订单不能为空");
        }
        let mut acc = Money::from_cents(0)?;
        for line in &self.lines {
            acc = acc.checked_add(line.line_total()?)?;
        }
        Ok(acc)
    }

    /// 两个实体 ID 不同则永不相等（即使金额相同）
    fn same_identity(&self, other: &Order) -> bool {
        self.id == other.id
    }
}

fn demo_naive_draft() -> OrderDraft {
    let mut d = OrderDraft::default();
    d.lines.push(("BOOK-01".into(), 1999, 1));
    d.lines.push(("BOOK-02".into(), 2999, 2));
    d
}

fn build_sample_order() -> Result<Order, &'static str> {
    let mut order = Order::new(OrderId(1001));
    let sku = Sku::new("book-rust-01")?;
    let price = Money::from_cents(4999)?;
    order.add_line(sku, price, 1)?;
    let sku2 = Sku::new("book-rust-02")?;
    order.add_line(sku2, Money::from_cents(3999)?, 2)?;
    Ok(order)
}

fn run_tests() {
    // 值对象：相等按值
    let m1 = Money::from_cents(100).unwrap();
    let m2 = Money::from_cents(100).unwrap();
    assert_eq!(m1, m2);
    assert_eq!(m1.checked_add(m2).unwrap().cents(), 200);

    let sku_a = Sku::new("abc").unwrap();
    let sku_b = Sku::new(" abc ").unwrap();
    assert_eq!(sku_a, sku_b);

    assert!(Sku::new("").is_err());

    // 直观版 draft
    let draft = demo_naive_draft();
    assert_eq!(draft.naive_total_cents(), 1999 + 2999 * 2);

    // 进阶版聚合
    let order = build_sample_order().unwrap();
    assert_eq!(order.line_count(), 2);
    let sub = order.subtotal().unwrap();
    assert_eq!(sub.cents(), 4999 + 3999 * 2);

    let empty = Order::new(OrderId(2));
    assert!(empty.subtotal().is_err());

    let mut bad = Order::new(OrderId(3));
    assert!(
        bad.add_line(Sku::new("x").unwrap(), Money::from_cents(10).unwrap(), 0).is_err()
    );

    let o1 = Order::new(OrderId(1));
    let o2 = Order::new(OrderId(2));
    assert!(o1.same_identity(&o1));
    assert!(!o1.same_identity(&o2));

    let mut dup_sku_order = Order::new(OrderId(9));
    let sku = Sku::new("dup").unwrap();
    dup_sku_order
        .add_line(sku.clone(), Money::from_cents(100).unwrap(), 1)
        .unwrap();
    dup_sku_order
        .add_line(sku, Money::from_cents(100).unwrap(), 1)
        .unwrap();
    assert_eq!(dup_sku_order.line_count(), 2);

    println!("✅ ddd_tactical 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ DDD 战术：实体 / 值对象 / 聚合根 ************\n");

    let draft = demo_naive_draft();
    println!("直观版 OrderDraft 合计（分）: {}", draft.naive_total_cents());

    match build_sample_order() {
        Ok(order) => {
            println!("聚合根 Order {:?} 行数: {}", order.id(), order.line_count());
            if let Ok(total) = order.subtotal() {
                println!("小计: {} USD（教学用）", total);
            }
        }
        Err(e) => println!("构建订单失败: {e}"),
    }

    run_tests();
}

fn main() {
    bootstrap();
}