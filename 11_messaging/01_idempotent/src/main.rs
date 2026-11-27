/*
[主题]: 至少一次投递 + 消费者幂等 —— 重复消息不应重复扣款
[难度]: Medium
[前置]: `11_messaging/00_async`（Broker 心智）、`03_error_collections`

一、主题说明
- **至少一次（at-least-once）**：Broker 在未收到 ACK 前会重投；网络抖动、进程崩溃都可能导致**同一条业务消息被处理两次**。
- **幂等**：用 `message_id` 或业务键（如 `order_id`）记录「已处理」，第二次到达时直接 ACK，不再改状态。
- 本课内存 Broker 可 `redeliver` 模拟重投；**不连 NATS/Kafka**。

二、写法对比
1. 直观版: `charge_once` 每收到一条就扣款，重投即多扣。
2. 进阶版: `IdempotentLedger` + `ProcessedStore`（类似 DB 唯一索引 / Redis SETNX）。

三、真实用途
- 支付、发券、开通权益 —— 必须幂等；Kafka 常配合 `consumer offset` + 业务去重表。
资料: https://nats.io/docs/
      https://docs.rs/rdkafka/latest/rdkafka/
      https://www.youtube.com/@RustVideos
*/

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

// ── 消息与账本 ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct PaymentCommand {
    /// Broker 侧消息 ID（重投时不变）
    message_id: String,
    order_id: u64,
    user_id: u64,
    amount_cents: u64,
}

#[derive(Debug, Default, Clone)]
struct Wallet {
    balance_cents: i64,
    /// 记录每笔订单扣了多少钱（便于断言）
    debits_by_order: HashMap<u64, u64>,
}

impl Wallet {
    fn debit_for_order(&mut self, order_id: u64, amount: u64) {
        self.balance_cents -= amount as i64;
        *self.debits_by_order.entry(order_id).or_insert(0) += amount;
    }
}

// ── 假 Broker：支持 ACK 与故意不 ACK 导致重投 ───────────────────

#[derive(Debug, Clone)]
struct StoredMessage {
    delivery_tag: u64,
    payload: PaymentCommand,
    delivered: bool,
}

#[derive(Debug, PartialEq)]
enum BrokerError {
    NotFound,
    AlreadyAcked,
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::NotFound => write!(f, "delivery_tag 不存在"),
            BrokerError::AlreadyAcked => write!(f, "消息已 ACK"),
        }
    }
}

/// 简化「未 ACK 可重投」队列
struct InMemoryAtLeastOnceBroker {
    next_tag: u64,
    inbox: VecDeque<StoredMessage>,
    inflight: HashMap<u64, StoredMessage>,
    acked_tags: HashSet<u64>,
}

impl InMemoryAtLeastOnceBroker {
    fn new() -> Self {
        Self {
            next_tag: 1,
            inbox: VecDeque::new(),
            inflight: HashMap::new(),
            acked_tags: HashSet::new(),
        }
    }

    fn enqueue(&mut self, cmd: PaymentCommand) {
        let tag = self.next_tag;
        self.next_tag += 1;
        self.inbox.push_back(StoredMessage {
            delivery_tag: tag,
            payload: cmd,
            delivered: false,
        });
    }

    /// consumer `poll`：移入 inflight，等待 ACK
    fn poll(&mut self) -> Option<StoredMessage> {
        let mut msg = self.inbox.pop_front()?;
        msg.delivered = true;
        let tag = msg.delivery_tag;
        self.inflight.insert(tag, msg.clone());
        Some(msg)
    }

    fn ack(&mut self, delivery_tag: u64) -> Result<(), BrokerError> {
        if self.acked_tags.contains(&delivery_tag) {
            return Err(BrokerError::AlreadyAcked);
        }
        match self.inflight.remove(&delivery_tag) {
            Some(_) => {
                self.acked_tags.insert(delivery_tag);
                Ok(())
            }
            None => Err(BrokerError::NotFound),
        }
    }

    /// 未 ACK 时 Broker 重投（同 payload、同 message_id）
    fn redeliver_all_inflight(&mut self) {
        let mut tags: Vec<u64> = self.inflight.keys().copied().collect();
        tags.sort_unstable();
        for tag in tags {
            if let Some(mut msg) = self.inflight.remove(&tag) {
                msg.delivered = false;
                self.inbox.push_back(msg);
            }
        }
    }

    fn inflight_count(&self) -> usize {
        self.inflight.len()
    }
}

// ── 直观版消费者 ────────────────────────────────────────────────

struct NaivePaymentConsumer {
    wallet: Wallet,
    processed_deliveries: u32,
}

impl NaivePaymentConsumer {
    fn new() -> Self {
        Self {
            wallet: Wallet::default(),
            processed_deliveries: 0,
        }
    }

    /// 每条消息都扣款 —— 重投即重复扣款（BUG）
    fn handle(&mut self, msg: &StoredMessage) {
        self.processed_deliveries += 1;
        let p = &msg.payload;
        self.wallet
            .debit_for_order(p.order_id, p.amount_cents);
    }
}

// ── 进阶：幂等键 = message_id（也可用 order_id 做业务幂等）──────

struct ProcessedStore {
    seen_message_ids: HashSet<String>,
}

impl ProcessedStore {
    fn new() -> Self {
        Self {
            seen_message_ids: HashSet::new(),
        }
    }

    fn mark_if_new(&mut self, message_id: &str) -> bool {
        self.seen_message_ids.insert(message_id.to_string())
    }

    fn contains(&self, message_id: &str) -> bool {
        self.seen_message_ids.contains(message_id)
    }
}

struct IdempotentPaymentConsumer {
    wallet: Wallet,
    store: ProcessedStore,
    applied: u32,
    skipped_duplicates: u32,
}

impl IdempotentPaymentConsumer {
    fn new() -> Self {
        Self {
            wallet: Wallet::default(),
            store: ProcessedStore::new(),
            applied: 0,
            skipped_duplicates: 0,
        }
    }

    fn handle(&mut self, msg: &StoredMessage) {
        let key = &msg.payload.message_id;
        if !self.store.mark_if_new(key) {
            self.skipped_duplicates += 1;
            return;
        }
        self.applied += 1;
        let p = &msg.payload;
        self.wallet
            .debit_for_order(p.order_id, p.amount_cents);
    }
}

fn run_tests() {
    let cmd = PaymentCommand {
        message_id: "msg-pay-001".into(),
        order_id: 9001,
        user_id: 3,
        amount_cents: 2500,
    };

    // 直观版：同一条消息处理两次 → 扣两次
    let mut naive = NaivePaymentConsumer::new();
    let msg1 = StoredMessage {
        delivery_tag: 1,
        payload: cmd.clone(),
        delivered: true,
    };
    let msg2 = StoredMessage {
        delivery_tag: 2,
        payload: cmd.clone(),
        delivered: true,
    };
    naive.handle(&msg1);
    naive.handle(&msg2);
    assert_eq!(naive.wallet.balance_cents, -5000);
    assert_eq!(
        naive.wallet.debits_by_order.get(&9001),
        Some(&5000),
        "重复投递导致重复扣款"
    );

    // 幂等版：两次相同 message_id 只扣一次
    let mut idem = IdempotentPaymentConsumer::new();
    idem.handle(&msg1);
    idem.handle(&msg2);
    assert_eq!(idem.wallet.balance_cents, -2500);
    assert_eq!(idem.skipped_duplicates, 1);
    assert_eq!(idem.applied, 1);
    assert!(idem.store.contains("msg-pay-001"));

    // Broker 重投流程
    let mut broker = InMemoryAtLeastOnceBroker::new();
    broker.enqueue(cmd.clone());
    let first = broker.poll().unwrap();
    assert_eq!(broker.inflight_count(), 1);

    let mut consumer = IdempotentPaymentConsumer::new();
    consumer.handle(&first);
    assert_eq!(consumer.wallet.balance_cents, -2500);

    // 模拟消费者宕机未 ACK
    broker.redeliver_all_inflight();
    assert_eq!(broker.inflight_count(), 0);
    let redelivered = broker.poll().unwrap();
    assert_eq!(redelivered.payload.message_id, cmd.message_id);
    consumer.handle(&redelivered);
    assert_eq!(
        consumer.wallet.balance_cents, -2500,
        "幂等后重投不应再扣款"
    );
    assert_eq!(consumer.skipped_duplicates, 1);

    broker.ack(redelivered.delivery_tag).unwrap();
    assert_eq!(
        broker.ack(redelivered.delivery_tag),
        Err(BrokerError::AlreadyAcked)
    );
    assert!(broker.ack(99999).is_err());

    println!("✅ 至少一次投递 + 幂等消费 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 至少一次 + 幂等 ************\n");

    let cmd = PaymentCommand {
        message_id: "demo-ack-1".into(),
        order_id: 77,
        user_id: 1,
        amount_cents: 999,
    };

    println!("【直观版】两条相同业务消息 → 余额:");
    let mut naive = NaivePaymentConsumer::new();
    for tag in 1..=2 {
        naive.handle(&StoredMessage {
            delivery_tag: tag,
            payload: cmd.clone(),
            delivered: true,
        });
    }
    println!("  balance_cents = {}", naive.wallet.balance_cents);

    println!("\n【进阶版】Broker 重投 + IdempotentConsumer:");
    let mut broker = InMemoryAtLeastOnceBroker::new();
    broker.enqueue(cmd.clone());
    let m = broker.poll().unwrap();

    let mut idem = IdempotentPaymentConsumer::new();
    idem.handle(&m);
    println!("  首次处理余额: {}", idem.wallet.balance_cents);

    broker.redeliver_all_inflight();
    if let Some(m2) = broker.poll() {
        idem.handle(&m2);
        let _ = broker.ack(m2.delivery_tag);
    }
    println!(
        "  重投后余额: {}（跳过重复 {} 次）",
        idem.wallet.balance_cents, idem.skipped_duplicates
    );

    println!("\n生产建议: message_id 入库唯一约束，或 order_id 状态机只允许一次「已支付」迁移。");
    println!("Kafka 消费者组: https://docs.rs/rdkafka/latest/rdkafka/");
    println!("NATS JetStream ACK: https://nats.io/docs/\n");

    run_tests();
}

fn main() {
    bootstrap();
}