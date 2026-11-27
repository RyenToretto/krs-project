/*
[主题]: 消息队列解耦 —— HTTP 立刻返回，副作用进内存 Broker
[难度]: Easy → Medium
[前置]: `08_web_axum`（Handler 心智）、`07_async_tokio`（可选，本课用同步队列模拟「异步解耦」）

一、主题说明
- **解耦**：下单接口只负责校验 + 入队，不负责发邮件、扣库存、打埋点（避免接口变慢、失败级联）。
- 本课用 **内存假 Broker**（`VecDeque` + `HashMap<topic, queue>`），不连接 NATS / Kafka，离线可跑。
- 生产对照：Handler `publish` → NATS subject / Kafka topic → 独立 consumer 进程处理。

二、写法对比
1. 直观版: `CheckoutService` 在同一线程里顺序调用邮件、库存、分析，任意一步失败拖垮整单。
2. 进阶版: `InMemoryBroker` + `EventPublisher` trait，`place_order` 只 `publish("order.created", ...)`，Worker `drain` 后分发给多个 `EventHandler`。

三、真实用途
- 削峰填谷、跨团队边界、失败后重试（下一课讲至少一次 + 幂等）。
资料: https://nats.io/docs/
      https://docs.rs/rdkafka/latest/rdkafka/
      https://www.youtube.com/@RustVideos
*/

use std::collections::{HashMap, VecDeque};
use std::fmt;

// ── 领域事件 ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderCreated {
    order_id: u64,
    user_id: u64,
    amount_cents: u64,
}

impl OrderCreated {
    fn encode(&self) -> String {
        format!("{}:{}:{}", self.order_id, self.user_id, self.amount_cents)
    }

    fn decode(raw: &str) -> Option<Self> {
        let mut parts = raw.split(':');
        let order_id: u64 = parts.next()?.parse().ok()?;
        let user_id: u64 = parts.next()?.parse().ok()?;
        let amount_cents: u64 = parts.next()?.parse().ok()?;
        Some(Self {
            order_id,
            user_id,
            amount_cents,
        })
    }
}

// ── 副作用计数器（模拟下游系统被调用了几次）──────────────────────

#[derive(Debug, Default, Clone)]
struct SideEffectLog {
    emails: u32,
    inventory: u32,
    analytics: u32,
}

impl SideEffectLog {
    fn total(&self) -> u32 {
        self.emails + self.inventory + self.analytics
    }
}

// ── 直观版：全部同步串行 ─────────────────────────────────────────

struct NaiveCheckout {
    log: SideEffectLog,
}

impl NaiveCheckout {
    fn new() -> Self {
        Self {
            log: SideEffectLog::default(),
        }
    }

    /// 模拟 HTTP 请求线程：三件事都在本函数里做完才「返回」
    fn place_order_sync(&mut self, order: OrderCreated) -> Result<(), String> {
        self.send_email(&order)?;
        self.reserve_inventory(&order)?;
        self.track_analytics(&order)?;
        Ok(())
    }

    fn send_email(&mut self, _o: &OrderCreated) -> Result<(), String> {
        self.log.emails += 1;
        Ok(())
    }

    fn reserve_inventory(&mut self, _o: &OrderCreated) -> Result<(), String> {
        self.log.inventory += 1;
        Ok(())
    }

    fn track_analytics(&mut self, _o: &OrderCreated) -> Result<(), String> {
        self.log.analytics += 1;
        Ok(())
    }
}

// ── 进阶：假 Broker + Handler 分派 ───────────────────────────────

const TOPIC_ORDER_CREATED: &str = "order.created";

#[derive(Debug, Clone)]
struct Envelope {
    seq: u64,
    topic: String,
    body: String,
}

#[derive(Debug, PartialEq)]
enum BrokerError {
    QueueFull { topic: String, cap: usize },
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::QueueFull { topic, cap } => {
                write!(f, "topic {} 队列已满（上限 {}）", topic, cap)
            }
        }
    }
}

/// 发布端契约 —— 真项目里可能是 NATS `Client::publish` / rdkafka `FutureProducer`
trait EventPublisher {
    fn publish(&mut self, topic: &str, body: impl Into<String>) -> Result<u64, BrokerError>;
}

/// 内存 Broker：按 topic 分队列，有界防止 OOM
struct InMemoryBroker {
    next_seq: u64,
    cap_per_topic: usize,
    queues: HashMap<String, VecDeque<Envelope>>,
}

impl InMemoryBroker {
    fn new(cap_per_topic: usize) -> Self {
        Self {
            next_seq: 1,
            cap_per_topic,
            queues: HashMap::new(),
        }
    }

    fn pending(&self, topic: &str) -> usize {
        self.queues.get(topic).map(|q| q.len()).unwrap_or(0)
    }

    /// Worker 拉取一条（模拟 consumer poll）
    fn poll_one(&mut self, topic: &str) -> Option<Envelope> {
        self.queues.get_mut(topic)?.pop_front()
    }

    fn drain_topic(&mut self, topic: &str, max: usize) -> Vec<Envelope> {
        let mut out = Vec::new();
        for _ in 0..max {
            match self.poll_one(topic) {
                Some(env) => out.push(env),
                None => break,
            }
        }
        out
    }
}

impl EventPublisher for InMemoryBroker {
    fn publish(&mut self, topic: &str, body: impl Into<String>) -> Result<u64, BrokerError> {
        let q = self.queues.entry(topic.to_string()).or_default();
        if q.len() >= self.cap_per_topic {
            return Err(BrokerError::QueueFull {
                topic: topic.to_string(),
                cap: self.cap_per_topic,
            });
        }
        let seq = self.next_seq;
        self.next_seq += 1;
        q.push_back(Envelope {
            seq,
            topic: topic.to_string(),
            body: body.into(),
        });
        Ok(seq)
    }
}

trait EventHandler {
    fn name(&self) -> &'static str;
    fn on_order_created(&mut self, order: &OrderCreated);
}

struct EmailHandler {
    log: SideEffectLog,
}

impl EventHandler for EmailHandler {
    fn name(&self) -> &'static str {
        "email"
    }

    fn on_order_created(&mut self, order: &OrderCreated) {
        let _ = order;
        self.log.emails += 1;
    }
}

struct InventoryHandler {
    log: SideEffectLog,
}

impl EventHandler for InventoryHandler {
    fn name(&self) -> &'static str {
        "inventory"
    }

    fn on_order_created(&mut self, order: &OrderCreated) {
        let _ = order;
        self.log.inventory += 1;
    }
}

struct AnalyticsHandler {
    log: SideEffectLog,
}

impl EventHandler for AnalyticsHandler {
    fn name(&self) -> &'static str {
        "analytics"
    }

    fn on_order_created(&mut self, order: &OrderCreated) {
        let _ = order;
        self.log.analytics += 1;
    }
}

/// 业务 API：只 publish，不直接碰下游
struct OrderApi<'a, P: EventPublisher> {
    broker: &'a mut P,
}

impl<'a, P: EventPublisher> OrderApi<'a, P> {
    fn place_order(&mut self, order: OrderCreated) -> Result<u64, BrokerError> {
        let body = order.encode();
        self.broker.publish(TOPIC_ORDER_CREATED, body)
    }
}

/// 后台 Worker：从 Broker 拉消息，广播给多个 Handler（扇出可用多 consumer，这里单进程演示）
struct OrderWorker<'a> {
    broker: &'a mut InMemoryBroker,
    handlers: Vec<&'a mut dyn EventHandler>,
}

impl<'a> OrderWorker<'a> {
    fn process_batch(&mut self, max: usize) -> usize {
        let batch = self.broker.drain_topic(TOPIC_ORDER_CREATED, max);
        let n = batch.len();
        for env in batch {
            let Some(order) = OrderCreated::decode(&env.body) else {
                continue;
            };
            for h in self.handlers.iter_mut() {
                h.on_order_created(&order);
            }
        }
        n
    }
}

fn run_tests() {
    // 直观版：一次下单触发 3 个同步副作用
    let mut naive = NaiveCheckout::new();
    naive
        .place_order_sync(OrderCreated {
            order_id: 1,
            user_id: 10,
            amount_cents: 9900,
        })
        .unwrap();
    assert_eq!(naive.log.total(), 3);

    // 进阶：下单瞬间 broker 里只有消息，副作用为 0
    let mut broker = InMemoryBroker::new(64);
    let mut email = EmailHandler {
        log: SideEffectLog::default(),
    };
    let mut inv = InventoryHandler {
        log: SideEffectLog::default(),
    };
    let mut analytics = AnalyticsHandler {
        log: SideEffectLog::default(),
    };

    {
        let mut api = OrderApi { broker: &mut broker };
        let seq = api
            .place_order(OrderCreated {
                order_id: 42,
                user_id: 7,
                amount_cents: 1200,
            })
            .unwrap();
        assert_eq!(seq, 1);
    }
    assert_eq!(broker.pending(TOPIC_ORDER_CREATED), 1);
    assert_eq!(email.log.total(), 0);

    let mut worker = OrderWorker {
        broker: &mut broker,
        handlers: vec![&mut email, &mut inv, &mut analytics],
    };
    assert_eq!(worker.process_batch(10), 1);
    assert_eq!(broker.pending(TOPIC_ORDER_CREATED), 0);
    assert_eq!(email.name(), "email");
    assert_eq!(email.log.emails, 1);
    assert_eq!(inv.log.inventory, 1);
    assert_eq!(analytics.log.analytics, 1);

    let mut peek_broker = InMemoryBroker::new(4);
    let seq = peek_broker.publish(TOPIC_ORDER_CREATED, "probe").unwrap();
    let env = peek_broker.poll_one(TOPIC_ORDER_CREATED).unwrap();
    assert_eq!(env.seq, seq);
    assert_eq!(env.topic, TOPIC_ORDER_CREATED);

    // 有界队列
    let mut small = InMemoryBroker::new(1);
    small.publish("t", "a").unwrap();
    assert!(matches!(
        small.publish("t", "b"),
        Err(BrokerError::QueueFull { .. })
    ));

    let roundtrip = OrderCreated {
        order_id: 9,
        user_id: 3,
        amount_cents: 50,
    };
    let decoded = OrderCreated::decode(&roundtrip.encode()).unwrap();
    assert_eq!(decoded, roundtrip);

    println!("✅ 异步解耦（内存 Broker）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 消息解耦：内存假 Broker ************\n");

    println!("【直观版】下单线程里同步做完所有副作用：");
    let mut naive = NaiveCheckout::new();
    naive
        .place_order_sync(OrderCreated {
            order_id: 100,
            user_id: 1,
            amount_cents: 500,
        })
        .unwrap();
    println!("  副作用计数: {:?}", naive.log);

    println!("\n【进阶版】API 只 publish，Worker 稍后 drain：");
    let mut broker = InMemoryBroker::new(32);
    let mut email = EmailHandler {
        log: SideEffectLog::default(),
    };
    let mut inv = InventoryHandler {
        log: SideEffectLog::default(),
    };
    let mut analytics = AnalyticsHandler {
        log: SideEffectLog::default(),
    };

    {
        let mut api = OrderApi { broker: &mut broker };
        let seq = api
            .place_order(OrderCreated {
                order_id: 200,
                user_id: 2,
                amount_cents: 1999,
            })
            .unwrap();
        println!("  HTTP 已返回，消息 seq={} 在 broker 排队", seq);
        println!("  队列深度: {}", broker.pending(TOPIC_ORDER_CREATED));
    }

    {
        let mut worker = OrderWorker {
            broker: &mut broker,
            handlers: vec![&mut email, &mut inv, &mut analytics],
        };
        let n = worker.process_batch(8);
        println!("  Worker 本轮处理 {} 条", n);
    }

    println!(
        "  各 Handler 日志: email={}, inventory={}, analytics={}",
        email.log.emails, inv.log.inventory, analytics.log.analytics
    );

    println!("\n对照生产：把 InMemoryBroker 换成 NATS / Kafka 客户端，Handler 可拆成独立二进制。");
    println!("NATS 文档: https://nats.io/docs/");
    println!("rdkafka: https://docs.rs/rdkafka/latest/rdkafka/\n");

    run_tests();
}

fn main() {
    bootstrap();
}