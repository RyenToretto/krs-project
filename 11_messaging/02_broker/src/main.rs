/*
[主题]: Broker 心智模型 —— Topic、订阅、Consumer Group、ACK/NACK
[难度]: Medium
[前置]: `11_messaging/00_async`、`11_messaging/01_idempotent`

一、主题说明
- **Broker** 介于发布者与订阅者之间：持久化（本课简化为内存日志）、路由（topic）、投递语义。
- **Consumer Group**：同组内一条消息通常只给一个成员（负载均衡）；不同组各自收到一份（广播）。
- **ACK/NACK**：成功则 ACK 删除 inflight；失败 NACK 可重回队列（至少一次的基础）。
- 对照 NATS Core/JetStream、Kafka partition + consumer group，但本课**纯内存、无外部依赖**。

二、写法对比
1. 直观版: 全局 `Vec` 回调列表，发布时 for 循环调用，无分组、无重试。
2. 进阶版: `MiniBroker` 维护 topic 日志、subscription、group 轮询分派、inflight + ack/nack。

三、真实用途
- 拆微服务、事件溯源入口、CDC 下游；Rust 常用 async-nats / rdkafka。
资料: https://nats.io/docs/
      https://docs.rs/rdkafka/latest/rdkafka/
      https://www.youtube.com/@RustVideos
*/

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

// ── 消息模型 ────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    offset: u64,
    topic: String,
    key: String,
    value: String,
}

#[derive(Debug, Clone)]
struct Delivery {
    delivery_tag: u64,
    record: Record,
    consumer_id: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct Subscription {
    topic: String,
    group: String,
    consumer_id: String,
}

// ── 直观版：广播回调 ────────────────────────────────────────────

struct NaiveBus {
    listeners: Vec<Box<dyn FnMut(&str)>>,
    deliveries: u32,
}

impl NaiveBus {
    fn new() -> Self {
        Self {
            listeners: Vec::new(),
            deliveries: 0,
        }
    }

    fn subscribe<F>(&mut self, f: F)
    where
        F: FnMut(&str) + 'static,
    {
        self.listeners.push(Box::new(f));
    }

    fn publish(&mut self, body: &str) {
        for listener in &mut self.listeners {
            listener(body);
            self.deliveries += 1;
        }
    }
}

// ── 进阶 MiniBroker ─────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum BrokerError {
    DeliveryNotFound,
    AlreadyAcked,
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::DeliveryNotFound => write!(f, "delivery 不存在"),
            BrokerError::AlreadyAcked => write!(f, "已 ACK"),
        }
    }
}

struct MiniBroker {
    next_offset: u64,
    next_delivery: u64,
    /// 每个 topic 只追加日志（类似 Kafka partition 简化版）
    logs: HashMap<String, Vec<Record>>,
    /// 每组每个 topic 记录读到哪里
    group_offsets: HashMap<(String, String), usize>,
    subscriptions: Vec<Subscription>,
    /// round-robin 游标
    rr_cursor: HashMap<(String, String), usize>,
    inflight: HashMap<u64, Delivery>,
    acked_tags: HashSet<u64>,
    stats: BrokerStats,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BrokerStats {
    published: u32,
    delivered: u32,
    acked: u32,
    nacked: u32,
}

impl MiniBroker {
    fn new() -> Self {
        Self {
            next_offset: 0,
            next_delivery: 1,
            logs: HashMap::new(),
            group_offsets: HashMap::new(),
            subscriptions: Vec::new(),
            rr_cursor: HashMap::new(),
            inflight: HashMap::new(),
            acked_tags: HashSet::new(),
            stats: BrokerStats::default(),
        }
    }

    fn register(&mut self, sub: Subscription) {
        self.subscriptions.push(sub);
    }

    fn publish(&mut self, topic: &str, key: &str, value: impl Into<String>) -> Record {
        self.next_offset += 1;
        let rec = Record {
            offset: self.next_offset,
            topic: topic.to_string(),
            key: key.to_string(),
            value: value.into(),
        };
        self.logs.entry(topic.to_string()).or_default().push(rec.clone());
        self.stats.published += 1;
        rec
    }

    fn log_len(&self, topic: &str) -> usize {
        self.logs.get(topic).map(|v| v.len()).unwrap_or(0)
    }

    /// 同 group 内轮询给一个 consumer（模拟 consumer group 分片）
    fn poll_for_group(&mut self, topic: &str, group: &str) -> Option<Delivery> {
        let members: Vec<_> = self
            .subscriptions
            .iter()
            .filter(|s| s.topic == topic && s.group == group)
            .cloned()
            .collect();
        if members.is_empty() {
            return None;
        }

        let offset_key = (group.to_string(), topic.to_string());
        let pos = self.group_offsets.get(&offset_key).copied().unwrap_or(0);
        let log = self.logs.get(topic)?;
        if pos >= log.len() {
            return None;
        }

        let idx_key = (group.to_string(), topic.to_string());
        let cursor = self.rr_cursor.entry(idx_key).or_insert(0);
        let member = &members[*cursor % members.len()];
        *cursor = cursor.saturating_add(1);

        let record = log[pos].clone();
        *self.group_offsets.entry(offset_key).or_insert(0) = pos + 1;

        let tag = self.next_delivery;
        self.next_delivery += 1;
        let delivery = Delivery {
            delivery_tag: tag,
            record,
            consumer_id: member.consumer_id.clone(),
        };
        self.inflight.insert(tag, delivery.clone());
        self.stats.delivered += 1;
        Some(delivery)
    }

    fn ack(&mut self, delivery_tag: u64) -> Result<(), BrokerError> {
        if self.acked_tags.contains(&delivery_tag) {
            return Err(BrokerError::AlreadyAcked);
        }
        match self.inflight.remove(&delivery_tag) {
            Some(_) => {
                self.acked_tags.insert(delivery_tag);
                self.stats.acked += 1;
                Ok(())
            }
            None => Err(BrokerError::DeliveryNotFound),
        }
    }

    /// NACK：消息回到 group offset 之前（简化：把 offset 减 1 并重投末尾）
    fn nack(&mut self, delivery_tag: u64) -> Result<(), BrokerError> {
        let delivery = match self.inflight.remove(&delivery_tag) {
            Some(d) => d,
            None => return Err(BrokerError::DeliveryNotFound),
        };
        self.stats.nacked += 1;
        let topic = delivery.record.topic.clone();
        let group = self
            .subscriptions
            .iter()
            .find(|s| s.consumer_id == delivery.consumer_id)
            .map(|s| s.group.clone())
            .unwrap_or_else(|| "default".into());
        let offset_key = (group, topic.clone());
        let pos = self.group_offsets.get(&offset_key).copied().unwrap_or(0);
        if pos > 0 {
            self.group_offsets.insert(offset_key, pos - 1);
        }
        Ok(())
    }

    fn inflight_count(&self) -> usize {
        self.inflight.len()
    }
}

/// 不同 group 各自消费全量（广播）；同 group 分担（负载均衡）
fn poll_until_empty(broker: &mut MiniBroker, topic: &str, group: &str, out: &mut Vec<Delivery>) {
    while let Some(d) = broker.poll_for_group(topic, group) {
        out.push(d);
    }
}

fn run_tests() {
    // 直观版：两个 listener 各收一次 → deliveries = 2
    let mut bus = NaiveBus::new();
    let seen_a = Rc::new(RefCell::new(0u32));
    let seen_b = Rc::new(RefCell::new(0u32));
    {
        let seen_a = Rc::clone(&seen_a);
        bus.subscribe(move |_| *seen_a.borrow_mut() += 1);
    }
    {
        let seen_b = Rc::clone(&seen_b);
        bus.subscribe(move |_| *seen_b.borrow_mut() += 1);
    }
    bus.publish("hello");
    assert_eq!(*seen_a.borrow(), 1);
    assert_eq!(*seen_b.borrow(), 1);
    assert_eq!(bus.deliveries, 2);

    // MiniBroker：发布 3 条到 orders
    let mut broker = MiniBroker::new();
    broker.register(Subscription {
        topic: "orders".into(),
        group: "fulfillment".into(),
        consumer_id: "worker-a".into(),
    });
    broker.register(Subscription {
        topic: "orders".into(),
        group: "fulfillment".into(),
        consumer_id: "worker-b".into(),
    });
    broker.register(Subscription {
        topic: "orders".into(),
        group: "analytics".into(),
        consumer_id: "metrics-1".into(),
    });

    broker.publish("orders", "k1", "o-1");
    broker.publish("orders", "k2", "o-2");
    broker.publish("orders", "k3", "o-3");
    assert_eq!(broker.log_len("orders"), 3);

    // fulfillment 组：3 条轮询分给 a/b
    let mut fulfill = Vec::new();
    poll_until_empty(&mut broker, "orders", "fulfillment", &mut fulfill);
    assert_eq!(fulfill.len(), 3);
    let workers: Vec<_> = fulfill.iter().map(|d| d.consumer_id.as_str()).collect();
    assert!(workers.contains(&"worker-a"));
    assert!(workers.contains(&"worker-b"));

    // analytics 组：独立 offset，仍可读 3 条（广播到另一 group）
    let mut analytics = Vec::new();
    poll_until_empty(&mut broker, "orders", "analytics", &mut analytics);
    assert_eq!(analytics.len(), 3);

    // ACK 清空 inflight（两个 group 各自 poll 过的都要 ack）
    for d in fulfill.iter().chain(analytics.iter()) {
        broker.ack(d.delivery_tag).unwrap();
    }
    assert_eq!(broker.inflight_count(), 0);
    assert_eq!(broker.ack(fulfill[0].delivery_tag), Err(BrokerError::AlreadyAcked));

    // NACK 把 offset 往回挪，可再 poll 一条
    broker.publish("orders", "k4", "o-4");
    let one = broker.poll_for_group("orders", "fulfillment").unwrap();
    broker.nack(one.delivery_tag).unwrap();
    let again = broker.poll_for_group("orders", "fulfillment").unwrap();
    assert_eq!(again.record.value, "o-4");

    assert!(broker.ack(404).is_err());
    assert_eq!(broker.stats.published, 4);

    println!("✅ Broker 心智（Topic / Group / ACK）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Broker 心智模型 ************\n");

    println!("【直观版】无 topic/group，纯回调广播:");
    let mut bus = NaiveBus::new();
    bus.subscribe(|msg| println!("  listener1: {}", msg));
    bus.subscribe(|msg| println!("  listener2: {}", msg));
    bus.publish("ping");

    println!("\n【进阶版】MiniBroker — orders topic:");
    let mut broker = MiniBroker::new();
    for id in ["worker-a", "worker-b"] {
        broker.register(Subscription {
            topic: "orders".into(),
            group: "ship".into(),
            consumer_id: id.into(),
        });
    }
    broker.register(Subscription {
        topic: "orders".into(),
        group: "audit".into(),
        consumer_id: "audit-1".into(),
    });

    let r = broker.publish("orders", "u:42", "下单成功");
    println!("  写入日志 offset={} value={}", r.offset, r.value);

    if let Some(d) = broker.poll_for_group("orders", "ship") {
        println!(
            "  ship 组 {} 领到: {}",
            d.consumer_id, d.record.value
        );
        let _ = broker.ack(d.delivery_tag);
    }
    if let Some(d) = broker.poll_for_group("orders", "audit") {
        println!("  audit 组 {} 领到: {}", d.consumer_id, d.record.value);
        let _ = broker.ack(d.delivery_tag);
    }

    println!("\n与生产对照:");
    println!("  NATS subject / queue group ≈ topic + consumer group");
    println!("  Kafka partition offset ≈ 本课 group_offsets + append log");
    println!("  https://nats.io/docs/");
    println!("  https://docs.rs/rdkafka/latest/rdkafka/");
    println!("  https://www.youtube.com/@RustVideos\n");

    run_tests();
}

fn main() {
    bootstrap();
}