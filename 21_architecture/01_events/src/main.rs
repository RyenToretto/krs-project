/*
[主题]: 领域事件 —— 聚合内产生、提交后分发、进程内总线
[难度]: Medium
[前置]: `21_architecture/00_ddd`（聚合根）；`11_messaging`（异步解耦心智）建议

一、主题说明
- **领域事件（Domain Event）**：领域中「已经发生的事实」，用过去式命名（`OrderPlaced`、`PaymentCaptured`）。
- 写在 **聚合内部**：状态变更成功后再 `record` 事件；持久化与发 MQ 应在 **同一事务或 Outbox** 后（本课用内存模拟）。
- **不要** 用事件做同步 RPC；事件用于最终一致性、审计、读模型投影。
- Rust 映射：`enum` 表达有限事件集合；`take_pending_events` 把缓冲交给应用层再分发。

二、写法对比
1. 直观版: `Vec<String>` 记录事件名 + 拼接 payload 字符串。
2. 进阶版: 强类型 `DomainEvent` + `InMemoryEventBus` 注册处理器，可测「谁收到几条」。

三、真实用途
- Axum handler 调应用服务 → 改聚合 → `drain events` → 投递 Kafka/NATS（另课）；审计表 INSERT。
资料: https://rust-lang.github.io/api-guidelines/
      https://nostarch.com/rust-rustaceans
      https://www.amazon.com/s?k=Rust+for+Rustaceans
*/

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// 进阶：强类型事件
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderId(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
enum DomainEvent {
    OrderCreated {
        order_id: OrderId,
        customer: String,
    },
    OrderLineAdded {
        order_id: OrderId,
        sku: String,
        quantity: u32,
    },
    OrderPlaced {
        order_id: OrderId,
        total_cents: u64,
    },
}

impl DomainEvent {
    fn name(&self) -> &'static str {
        match self {
            DomainEvent::OrderCreated { .. } => "OrderCreated",
            DomainEvent::OrderLineAdded { .. } => "OrderLineAdded",
            DomainEvent::OrderPlaced { .. } => "OrderPlaced",
        }
    }
}

impl fmt::Display for DomainEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// 聚合根：业务方法里 record，外人通过 `take_events` 取走
#[derive(Debug)]
struct OrderAggregate {
    id: OrderId,
    customer: String,
    lines: Vec<(String, u32)>,
    placed: bool,
    /// 未提交领域事件缓冲
    pending: Vec<DomainEvent>,
}

impl OrderAggregate {
    fn customer(&self) -> &str {
        &self.customer
    }

    fn create(id: u64, customer: &str) -> Result<Self, &'static str> {
        if customer.trim().is_empty() {
            return Err("客户名不能为空");
        }
        let order_id = OrderId(id);
        let agg = Self {
            id: order_id.clone(),
            customer: customer.trim().to_string(),
            lines: Vec::new(),
            placed: false,
            pending: vec![DomainEvent::OrderCreated {
                order_id,
                customer: customer.trim().to_string(),
            }],
        };
        Ok(agg)
    }

    fn add_line(&mut self, sku: &str, qty: u32) -> Result<(), &'static str> {
        if self.placed {
            return Err("已下单不可加行");
        }
        if sku.trim().is_empty() || qty == 0 {
            return Err("行项目非法");
        }
        let sku = sku.trim().to_string();
        self.lines.push((sku.clone(), qty));
        self.pending.push(DomainEvent::OrderLineAdded {
            order_id: self.id.clone(),
            sku,
            quantity: qty,
        });
        Ok(())
    }

    fn place(&mut self, total_cents: u64) -> Result<(), &'static str> {
        if self.placed {
            return Err("重复下单");
        }
        if self.lines.is_empty() {
            return Err("不能下空单");
        }
        self.placed = true;
        self.pending.push(DomainEvent::OrderPlaced {
            order_id: self.id.clone(),
            total_cents,
        });
        Ok(())
    }

    /// 取走并清空 pending —— 模拟 UoW 提交后发布
    fn take_pending_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.pending)
    }
}

// ---------------------------------------------------------------------------
// 直观版：字符串事件日志
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct NaiveEventLog {
    pub entries: Vec<String>,
}

impl NaiveEventLog {
    fn push(&mut self, name: &str, detail: &str) {
        self.entries.push(format!("{name}|{detail}"));
    }
}

// ---------------------------------------------------------------------------
// 进程内总线（教学用）
// ---------------------------------------------------------------------------

type Handler = fn(&DomainEvent);

#[derive(Debug, Default)]
struct InMemoryEventBus {
    handlers: HashMap<&'static str, Vec<Handler>>,
    delivered: usize,
    /// 处理器被调用的次数（模拟审计落库条数）
    handler_invocations: usize,
}

impl InMemoryEventBus {
    fn subscribe(&mut self, event_name: &'static str, h: Handler) {
        self.handlers.entry(event_name).or_default().push(h);
    }

    fn publish_all(&mut self, events: &[DomainEvent]) {
        for ev in events {
            if let Some(hs) = self.handlers.get(ev.name()) {
                for h in hs {
                    h(ev);
                    self.handler_invocations += 1;
                }
            }
            self.delivered += 1;
        }
    }

    fn delivered_count(&self) -> usize {
        self.delivered
    }

    fn handler_invocations(&self) -> usize {
        self.handler_invocations
    }
}

fn audit_handler(_ev: &DomainEvent) {
    // 生产环境写入 tracing / 审计表；本课由总线计数 invocation
}

fn run_tests() {
    let mut naive = NaiveEventLog::default();
    naive.push("OrderCreated", "id=1");
    assert_eq!(naive.entries.len(), 1);
    assert!(naive.entries[0].contains("OrderCreated"));

    let mut agg = OrderAggregate::create(42, "alice").unwrap();
    assert_eq!(agg.take_pending_events().len(), 1);

    agg.add_line("SKU-1", 2).unwrap();
    agg.add_line("SKU-2", 1).unwrap();
    agg.place(9999).unwrap();

    let batch = agg.take_pending_events();
    assert_eq!(batch.len(), 3);
    assert_eq!(batch.last(), Some(&DomainEvent::OrderPlaced {
        order_id: OrderId(42),
        total_cents: 9999,
    }));

    assert!(agg.take_pending_events().is_empty());

    let mut bus = InMemoryEventBus::default();
    bus.subscribe("OrderPlaced", audit_handler);

    let events = vec![DomainEvent::OrderPlaced {
        order_id: OrderId(1),
        total_cents: 100,
    }];
    bus.publish_all(&events);
    assert_eq!(bus.delivered_count(), 1);
    assert_eq!(bus.handler_invocations(), 1);

    let mut failed = OrderAggregate::create(2, "bob").unwrap();
    assert!(failed.place(1).is_err());

    println!("✅ domain_events 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 领域事件：记录、提交、分发 ************\n");

    let mut order = OrderAggregate::create(7, "rust-learner").unwrap();
    println!("客户: {}", order.customer());
    order.add_line("BOOK-DDD", 1).unwrap();
    order.place(5999).unwrap();

    let events = order.take_pending_events();
    println!("共产生 {} 条领域事件:", events.len());
    for (i, ev) in events.iter().enumerate() {
        println!("  {}. {ev:?}", i + 1);
    }

    let mut bus = InMemoryEventBus::default();
    bus.subscribe("OrderPlaced", audit_handler);
    bus.subscribe("OrderCreated", audit_handler);
    bus.publish_all(&events[1..]);

    run_tests();
}

fn main() {
    bootstrap();
}