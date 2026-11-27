/*
[主题]: 单元测试 —— 用 trait 做 Mock，隔离外部依赖
[难度]: Medium
[前置]: `04_traits_generics`、`09_persistence/00_repo`（仓储 trait）

一、主题说明
- 业务逻辑（`OrderService`）不应在单测里连真数据库或真 HTTP；把依赖抽成 **trait**，测试时注入 **假实现（Mock）**。
- Mock 要点：① 行为可配置（返回 Ok/Err）；② **记录调用**（调了几次、`user_id` 是多少），断言「是否按预期调了下游」。
- 生产可用 `mockall` 等库自动生成；本课手写 Mock，看清 trait object / 泛型边界怎么测。

二、写法对比
1. 直观版: `OrderService` 里直接 `HashMap` 查库存，测订单时顺带测了库存实现，失败难定位。
2. 进阶版: `trait Inventory` + `InMemoryInventory`（真） + `RecordingInventory`（Mock），`OrderService::checkout` 只依赖 trait。

三、真实用途
- sqlx / Redis / gRPC Client 都适用同一模式：`Repo` trait + 测试用 `FakeRepo`。
- 表驱动可同时测多组「库存足/不足 + Mock 调用次数」。
资料: https://doc.rust-lang.org/book/ch11-00-testing.html
      https://doc.rust-lang.org/rust-by-example/testing.html
*/

use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckoutError {
    UnknownSku(String),
    NotEnough { sku: String, need: u32, have: u32 },
}

/// 库存边界 —— 生产里可能是 Redis / 仓储 RPC。
trait Inventory {
    fn available(&self, sku: &str) -> Option<u32>;
    fn reserve(&mut self, sku: &str, qty: u32) -> Result<(), CheckoutError>;
}

struct InMemoryInventory {
    stock: std::collections::HashMap<String, u32>,
}

impl InMemoryInventory {
    fn with_items(items: &[(&str, u32)]) -> Self {
        let mut stock = std::collections::HashMap::new();
        for (k, v) in items {
            stock.insert((*k).to_string(), *v);
        }
        Self { stock }
    }
}

impl Inventory for InMemoryInventory {
    fn available(&self, sku: &str) -> Option<u32> {
        self.stock.get(sku).copied()
    }

    fn reserve(&mut self, sku: &str, qty: u32) -> Result<(), CheckoutError> {
        let have = self.stock.get(sku).copied();
        let Some(have) = have else {
            return Err(CheckoutError::UnknownSku(sku.to_string()));
        };
        if have < qty {
            return Err(CheckoutError::NotEnough {
                sku: sku.to_string(),
                need: qty,
                have,
            });
        }
        self.stock.insert(sku.to_string(), have - qty);
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ReserveCall {
    sku: String,
    qty: u32,
}

/// Mock：可预设 `available`，并在 `reserve` 时记录调用、按配置返回。
struct RecordingInventory {
    preset: std::collections::HashMap<String, u32>,
    calls: RefCell<Vec<ReserveCall>>,
    reserve_ok: bool,
}

impl RecordingInventory {
    fn new(preset: &[(&str, u32)], reserve_ok: bool) -> Self {
        let mut preset_map = std::collections::HashMap::new();
        for (k, v) in preset {
            preset_map.insert((*k).to_string(), *v);
        }
        Self {
            preset: preset_map,
            calls: RefCell::new(Vec::new()),
            reserve_ok,
        }
    }

    fn call_count(&self) -> usize {
        self.calls.borrow().len()
    }

    fn last_call(&self) -> Option<ReserveCall> {
        self.calls.borrow().last().cloned()
    }
}

impl Inventory for RecordingInventory {
    fn available(&self, sku: &str) -> Option<u32> {
        self.preset.get(sku).copied()
    }

    fn reserve(&mut self, sku: &str, qty: u32) -> Result<(), CheckoutError> {
        self.calls
            .borrow_mut()
            .push(ReserveCall {
                sku: sku.to_string(),
                qty,
            });
        if !self.reserve_ok {
            let have = self.preset.get(sku).copied().unwrap_or(0);
            return Err(CheckoutError::NotEnough {
                sku: sku.to_string(),
                need: qty,
                have,
            });
        }
        Ok(())
    }
}

struct OrderLine {
    sku: String,
    qty: u32,
}

/// 直观版：库存逻辑写死在服务里（难 Mock）。
struct NaiveCheckout {
    stock: std::collections::HashMap<String, u32>,
}

impl NaiveCheckout {
    fn place(&mut self, sku: &str, qty: u32) -> Result<(), CheckoutError> {
        let have = self.stock.get(sku).copied().unwrap_or(0);
        if have < qty {
            return Err(CheckoutError::NotEnough {
                sku: sku.to_string(),
                need: qty,
                have,
            });
        }
        self.stock.insert(sku.to_string(), have - qty);
        Ok(())
    }
}

/// 进阶版：只编排，库存交给 trait。
struct OrderService<I: Inventory> {
    inventory: I,
}

impl<I: Inventory> OrderService<I> {
    fn peek(&self, sku: &str) -> Option<u32> {
        self.inventory.available(sku)
    }

    fn checkout(&mut self, lines: &[OrderLine]) -> Result<(), CheckoutError> {
        for line in lines {
            let _ = self
                .inventory
                .available(&line.sku)
                .ok_or_else(|| CheckoutError::UnknownSku(line.sku.clone()))?;
            self.inventory.reserve(&line.sku, line.qty)?;
        }
        Ok(())
    }

    fn into_inventory(self) -> I {
        self.inventory
    }
}

#[derive(Debug, Clone, Copy)]
struct MockCase {
    name: &'static str,
    sku: &'static str,
    /// 是否在目录中（否 → `available` 为 None）
    in_catalog: bool,
    stock: u32,
    qty: u32,
    reserve_ok: bool,
    expect_ok: bool,
    expect_calls: usize,
}

const MOCK_CASES: &[MockCase] = &[
    MockCase {
        name: "库存足且 reserve 成功",
        sku: "BOOK-RUST",
        in_catalog: true,
        stock: 10,
        qty: 2,
        reserve_ok: true,
        expect_ok: true,
        expect_calls: 1,
    },
    MockCase {
        name: "未知 SKU",
        sku: "NO-SUCH",
        in_catalog: false,
        stock: 0,
        qty: 1,
        reserve_ok: true,
        expect_ok: false,
        expect_calls: 0,
    },
    MockCase {
        name: "reserve 返回不足",
        sku: "MOUSE",
        in_catalog: true,
        stock: 1,
        qty: 5,
        reserve_ok: false,
        expect_ok: false,
        expect_calls: 1,
    },
];

fn run_tests() {
    let mut naive = NaiveCheckout {
        stock: [("pen".to_string(), 3)].into_iter().collect(),
    };
    assert!(naive.place("pen", 2).is_ok());
    assert!(matches!(
        naive.place("pen", 99),
        Err(CheckoutError::NotEnough { .. })
    ));

    let mut real = OrderService {
        inventory: InMemoryInventory::with_items(&[("kb", 5)]),
    };
    assert!(real
        .checkout(&[OrderLine {
            sku: "kb".into(),
            qty: 1
        }])
        .is_ok());
    assert_eq!(real.peek("kb"), Some(4));

    for case in MOCK_CASES {
        let preset: Vec<(&str, u32)> = if case.in_catalog {
            vec![(case.sku, case.stock)]
        } else {
            vec![]
        };
        let mock = RecordingInventory::new(&preset, case.reserve_ok);
        let mut svc = OrderService { inventory: mock };
        let got = svc.checkout(&[OrderLine {
            sku: case.sku.into(),
            qty: case.qty,
        }]);
        let mock = svc.into_inventory();
        assert_eq!(
            got.is_ok(),
            case.expect_ok,
            "用例「{}」结果不符",
            case.name
        );
        assert_eq!(
            mock.call_count(),
            case.expect_calls,
            "用例「{}」reserve 调用次数",
            case.name
        );
    }

    println!("✅ trait Mock（记录调用 + 表驱动）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 trait Mock ************\n");

    let mut naive = NaiveCheckout {
        stock: [("sticker".into(), 100)].into_iter().collect(),
    };
    println!(
        "直观版 NaiveCheckout: place sticker x1 → {:?}",
        naive.place("sticker", 1)
    );

    let mock = RecordingInventory::new(&[("BOOK-RUST", 8)], true);
    let mut svc = OrderService { inventory: mock };
    let r = svc.checkout(&[OrderLine {
        sku: "BOOK-RUST".into(),
        qty: 3,
    }]);
    let mock = svc.into_inventory();
    println!("进阶版 Mock checkout: {:?}, reserve 调用 {} 次", r, mock.call_count());
    if let Some(c) = mock.last_call() {
        println!("  最后一次 reserve: {:?}", c);
    }

    println!("\n对照生产: Inventory trait ≈ 仓储接口；RecordingInventory ≈ 测试替身。\n");

    run_tests();
}

fn main() {
    bootstrap();
}