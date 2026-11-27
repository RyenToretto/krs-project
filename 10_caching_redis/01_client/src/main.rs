/*
[主题]: Redis 客户端心智 —— 命令、连接、过期（纯内存模拟，不用 redis crate）
[难度]: Medium
[前置]: `10_caching_redis/00_cache`、`07_async_tokio`（真 client 多为 async，本课先同步心智）

一、主题说明
- **Redis** 是内存键值库：字符串、哈希、列表等；后端拿它做缓存、会话、限流、分布式锁。
- Rust 官方文档推荐 `redis` crate：`Connection`、`Cmd`、或 async 下的 `MultiplexedConnection`。
- 本课实现 **MiniRedis**：支持 `GET`/`SET`/`SETEX`/`DEL`/`EXPIRE`/`PING`，外加 **假连接池** 借还连接。

二、写法对比
1. 直观版: 全局一个 `HashMap`，到处直接 `insert`/`get`，没有 TTL、没有连接抽象。
2. 进阶版: `RedisStore` + `RedisConnection` + `ConnectionPool`，命令返回 `RedisReply`，贴近官方 API 形状。

三、真实用途
- `redis::Client::open(url)?.get_connection()?` 然后 `redis::cmd("GET").arg(key).query()`。
- 生产常用连接池、`ConnectionManager`、集群分片；本课只建立「命令 + 池化借还」直觉。
资料: https://redis.io/docs/latest/develop/clients/rust/
      https://docs.rs/redis/latest/redis/
      https://docs.rs/redis/latest/redis/struct.Client.html
      https://www.youtube.com/@RustVideos
*/

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::time::{Duration, Instant};

// ── 回复与错误（类似 redis::RedisResult 的简化版）────────────────

#[derive(Debug, Clone, PartialEq)]
enum RedisReply {
    Status(String),
    BulkString(Option<String>),
    Integer(i64),
}

#[derive(Debug, PartialEq)]
enum RedisError {
    InvalidExpire,
    PoolExhausted,
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

type RedisResult<T> = Result<T, RedisError>;

// ── 带 TTL 的字符串值 ───────────────────────────────────────────

#[derive(Debug, Clone)]
struct RedisString {
    data: String,
    expire_at: Option<Instant>,
}

impl RedisString {
    fn alive(&self) -> bool {
        match self.expire_at {
            Some(t) => Instant::now() < t,
            None => true,
        }
    }
}

/// 单节点内存 Redis 引擎
#[derive(Debug, Default)]
struct RedisStore {
    strings: HashMap<String, RedisString>,
}

impl RedisStore {
    fn ping(&self) -> RedisReply {
        RedisReply::Status("PONG".into())
    }

    fn get(&mut self, key: &str) -> RedisReply {
        match self.strings.get(key) {
            Some(v) if v.alive() => RedisReply::BulkString(Some(v.data.clone())),
            Some(_) => {
                self.strings.remove(key);
                RedisReply::BulkString(None)
            }
            None => RedisReply::BulkString(None),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> RedisResult<RedisReply> {
        self.strings.insert(
            key.to_string(),
            RedisString {
                data: value.to_string(),
                expire_at: None,
            },
        );
        Ok(RedisReply::Status("OK".into()))
    }

    fn setex(&mut self, key: &str, seconds: u64, value: &str) -> RedisResult<RedisReply> {
        if seconds == 0 {
            return Err(RedisError::InvalidExpire);
        }
        let expire_at = Instant::now() + Duration::from_secs(seconds);
        self.strings.insert(
            key.to_string(),
            RedisString {
                data: value.to_string(),
                expire_at: Some(expire_at),
            },
        );
        Ok(RedisReply::Status("OK".into()))
    }

    fn del(&mut self, key: &str) -> RedisReply {
        let n = if self.strings.remove(key).is_some() {
            1
        } else {
            0
        };
        RedisReply::Integer(n)
    }

    fn expire(&mut self, key: &str, seconds: u64) -> RedisResult<RedisReply> {
        if seconds == 0 {
            return Err(RedisError::InvalidExpire);
        }
        match self.strings.get_mut(key) {
            Some(v) if v.alive() => {
                v.expire_at = Some(Instant::now() + Duration::from_secs(seconds));
                Ok(RedisReply::Integer(1))
            }
            Some(_) => {
                self.strings.remove(key);
                Ok(RedisReply::Integer(0))
            }
            None => Ok(RedisReply::Integer(0)),
        }
    }
}

// ── 单节点 + 连接池槽位（拥有 Store，避免多重可变借用）────────────
// 真 Redis：`Client` 拥有连接配置；`get_connection()` 借 TCP 句柄发命令。

struct RedisNode {
    store: RedisStore,
    available_slots: VecDeque<u32>,
    capacity: usize,
}

impl RedisNode {
    fn with_pool(capacity: usize) -> Self {
        Self {
            store: RedisStore::default(),
            available_slots: (1..=capacity as u32).collect(),
            capacity,
        }
    }

    /// 借连接槽 —— 对应 `pool.get()` / `get_multiplexed_connection()`
    fn checkout_slot(&mut self) -> RedisResult<u32> {
        self.available_slots
            .pop_front()
            .ok_or(RedisError::PoolExhausted)
    }

    fn checkin_slot(&mut self, slot_id: u32) {
        debug_assert!(slot_id >= 1 && slot_id <= self.capacity as u32);
        self.available_slots.push_back(slot_id);
    }

    fn cmd_ping(&self) -> RedisReply {
        self.store.ping()
    }

    fn cmd_get(&mut self, key: &str) -> RedisReply {
        self.store.get(key)
    }

    fn cmd_set(&mut self, key: &str, value: &str) -> RedisResult<RedisReply> {
        self.store.set(key, value)
    }

    fn cmd_setex(&mut self, key: &str, sec: u64, value: &str) -> RedisResult<RedisReply> {
        self.store.setex(key, sec, value)
    }

    fn cmd_del(&mut self, key: &str) -> RedisReply {
        self.store.del(key)
    }

    fn store_mut(&mut self) -> &mut RedisStore {
        &mut self.store
    }
}

// ── 直观版：全局 map ─────────────────────────────────────────────

#[derive(Default)]
struct NaiveGlobalCache {
    data: HashMap<String, String>,
}

impl NaiveGlobalCache {
    fn put(&mut self, k: &str, v: &str) {
        self.data.insert(k.to_string(), v.to_string());
    }

    fn get(&self, k: &str) -> Option<&str> {
        self.data.get(k).map(|s| s.as_str())
    }
}

fn run_tests() {
    let mut naive = NaiveGlobalCache::default();
    naive.put("session:1", "alice");
    assert_eq!(naive.get("session:1"), Some("alice"));

    let mut node = RedisNode::with_pool(2);

    let slot1 = node.checkout_slot().unwrap();
    assert_eq!(node.cmd_ping(), RedisReply::Status("PONG".into()));
    node.cmd_set("user:42", "bob").unwrap();
    assert_eq!(
        node.cmd_get("user:42"),
        RedisReply::BulkString(Some("bob".into()))
    );
    node.checkin_slot(slot1);

    let slot2 = node.checkout_slot().unwrap();
    assert_ne!(slot2, 0);
    node.cmd_setex("token:abc", 60, "secret").unwrap();
    node.store_mut().expire("token:abc", 120).unwrap();
    assert_eq!(
        node.cmd_get("token:abc"),
        RedisReply::BulkString(Some("secret".into()))
    );
    node.checkin_slot(slot2);

    // 池容量 2：借满再借应失败
    let h1 = node.checkout_slot().unwrap();
    let h2 = node.checkout_slot().unwrap();
    assert!(matches!(
        node.checkout_slot(),
        Err(RedisError::PoolExhausted)
    ));
    node.checkin_slot(h2);
    node.checkin_slot(h1);

    // 过期键
    node.store_mut().strings.insert(
        "gone".into(),
        RedisString {
            data: "x".into(),
            expire_at: Some(Instant::now() - Duration::from_secs(1)),
        },
    );
    assert_eq!(node.cmd_get("gone"), RedisReply::BulkString(None));
    assert!(!node.store_mut().strings.contains_key("gone"));

    let slot3 = node.checkout_slot().unwrap();
    node.cmd_del("user:42");
    assert_eq!(
        node.cmd_get("user:42"),
        RedisReply::BulkString(None)
    );
    node.checkin_slot(slot3);

    // 同一借出的槽位上连续发令（pipeline 心智）
    let slot4 = node.checkout_slot().unwrap();
    node.cmd_set("k1", "v1").unwrap();
    node.cmd_set("k2", "v2").unwrap();
    assert_eq!(
        node.cmd_get("k1"),
        RedisReply::BulkString(Some("v1".into()))
    );
    node.checkin_slot(slot4);

    assert_eq!(slot1, 1);

    println!("✅ Redis 客户端心智（内存模拟）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Redis Client Mental ************\n");

    let mut node = RedisNode::with_pool(3);

    println!("从池里借连接槽 → PING → SETEX session → GET");
    let slot = node.checkout_slot().unwrap();
    println!("  连接槽 id={}", slot);
    println!("  PING → {:?}", node.cmd_ping());
    let _ = node.cmd_setex("session:demo", 300, "user_id=7");
    println!("  GET session:demo → {:?}", node.cmd_get("session:demo"));
    node.checkin_slot(slot);

    let naive = NaiveGlobalCache::default();
    println!(
        "\n直观版全局 HashMap 无法表达 TTL/连接池，只适合理解「键值」本质。"
    );
    let _ = naive;

    println!("\n对照 docs.rs/redis: Client → get_connection / MultiplexedConnection");
    println!("https://docs.rs/redis/latest/redis/\n");

    run_tests();
}

fn main() {
    bootstrap();
}