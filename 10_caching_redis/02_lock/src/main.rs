/*
[主题]: 分布式锁入门 —— SET NX EX + 唯一 token（内存模拟 Redlock 核心思想）
[难度]: Medium
[前置]: `10_caching_redis/01_client`、`06_std_concurrency`（对比本地 Mutex）

一、主题说明
- **分布式锁**：多台机器 / 多个进程要互斥访问共享资源（库存扣减、定时任务只跑一份）。
- Redis 常见写法：`SET lock_key unique_token NX EX 30` —— 只有不存在才设成功，并带过期防死锁。
- 释放时要 **校验 token 再删**（Lua 脚本原子 compare-and-del），避免删掉别人的锁。
- 本课用 **MiniRedisLockStore** 模拟，不连真 Redis；单机 `std::sync::Mutex` 对照说明局限。

二、写法对比
1. 直观版: `static` + `Mutex` 只在本进程内有效，多实例部署会各锁各的。
2. 进阶版: `DistributedLock::try_acquire` 返回 owner token；`release` 带 token 校验；支持 renew。

三、真实用途与陷阱
- 生产用 `redis` crate + Lua，或 Redisson 风格库；过期时间要大于业务临界区，且考虑续约。
- 勿只用 `SETNX` 不设 TTL：进程崩溃会永久死锁。
- 更复杂场景见 Redlock 辩论；就业先掌握单 Redis 实例 + token 模式即可。
资料: https://redis.io/docs/latest/develop/clients/rust/
      https://docs.rs/redis/latest/redis/
      https://redis.io/docs/latest/develop/use/patterns/distributed-locks/
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ── 锁存储错误 ───────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
enum LockError {
    NotHeld,
    WrongOwner,
    AlreadyHeld,
    InvalidTtl,
    StoreBusy,
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

type LockResult<T> = Result<T, LockError>;

// ── Redis 里的锁记录：值 = owner token，带过期 ───────────────────

#[derive(Debug, Clone)]
struct LockRecord {
    owner: String,
    expire_at: Instant,
}

impl LockRecord {
    fn alive(&self) -> bool {
        Instant::now() < self.expire_at
    }
}

/// 模拟 Redis：`SET key token NX EX seconds`
#[derive(Debug, Default)]
struct MiniRedisLockStore {
    locks: HashMap<String, LockRecord>,
}

impl MiniRedisLockStore {
    /// `SET resource_lock token NX EX ttl` —— 成功返回 true
    fn set_nx_ex(&mut self, key: &str, owner: &str, ttl: Duration) -> LockResult<bool> {
        if ttl.is_zero() {
            return Err(LockError::InvalidTtl);
        }
        self.purge_expired(key);
        if let Some(rec) = self.locks.get(key) {
            if rec.alive() {
                return Ok(false);
            }
        }
        self.locks.insert(
            key.to_string(),
            LockRecord {
                owner: owner.to_string(),
                expire_at: Instant::now() + ttl,
            },
        );
        Ok(true)
    }

    /// Lua 心智：if get == token then del end
    fn release_if_owner(&mut self, key: &str, expected_owner: &str) -> LockResult<bool> {
        self.purge_expired(key);
        match self.locks.get(key) {
            Some(rec) if rec.alive() && rec.owner == expected_owner => {
                self.locks.remove(key);
                Ok(true)
            }
            Some(rec) if rec.alive() => Err(LockError::WrongOwner),
            _ => Err(LockError::NotHeld),
        }
    }

    fn renew(&mut self, key: &str, owner: &str, ttl: Duration) -> LockResult<()> {
        if ttl.is_zero() {
            return Err(LockError::InvalidTtl);
        }
        self.purge_expired(key);
        match self.locks.get(key) {
            Some(rec) if rec.alive() && rec.owner == owner => {
                let entry = self.locks.get_mut(key).expect("just matched");
                entry.expire_at = Instant::now() + ttl;
                Ok(())
            }
            Some(_) => Err(LockError::WrongOwner),
            None => Err(LockError::NotHeld),
        }
    }

    fn purge_expired(&mut self, key: &str) {
        if let Some(rec) = self.locks.get(key) {
            if !rec.alive() {
                self.locks.remove(key);
            }
        }
    }

}

// ── 进阶：分布式锁句柄 ───────────────────────────────────────────

#[derive(Debug, Clone)]
struct LockGuard {
    key: String,
    token: String,
}

struct DistributedLock<'a> {
    store: &'a mut MiniRedisLockStore,
    default_ttl: Duration,
}

impl<'a> DistributedLock<'a> {
    fn new(store: &'a mut MiniRedisLockStore, default_ttl: Duration) -> Self {
        Self {
            store,
            default_ttl,
        }
    }

    fn try_acquire(&mut self, resource: &str, token: &str) -> LockResult<Option<LockGuard>> {
        let key = format!("lock:{resource}");
        if self.store.set_nx_ex(&key, token, self.default_ttl)? {
            Ok(Some(LockGuard {
                key,
                token: token.to_string(),
            }))
        } else {
            Ok(None)
        }
    }

    fn release(&mut self, guard: LockGuard) -> LockResult<()> {
        let ok = self
            .store
            .release_if_owner(&guard.key, &guard.token)?;
        if ok {
            Ok(())
        } else {
            Err(LockError::NotHeld)
        }
    }

    fn renew(&mut self, guard: &LockGuard, ttl: Duration) -> LockResult<()> {
        self.store.renew(&guard.key, &guard.token, ttl)
    }
}

// ── 直观版：进程内 Mutex（不能跨实例）────────────────────────────

static NAIVE_COUNTER: Mutex<u32> = Mutex::new(0);

fn naive_increment_local() -> LockResult<u32> {
    let mut g = NAIVE_COUNTER.lock().map_err(|_| LockError::StoreBusy)?;
    *g += 1;
    Ok(*g)
}

// ── 业务：秒杀扣库存（必须持锁）────────────────────────────────

struct InventoryService<'a> {
    stock: u32,
    lock: DistributedLock<'a>,
}

impl<'a> InventoryService<'a> {
    fn try_deduct(&mut self, sku: &str, buyer_token: &str, qty: u32) -> LockResult<bool> {
        let Some(guard) = self.lock.try_acquire(sku, buyer_token)? else {
            return Err(LockError::AlreadyHeld);
        };
        if self.stock < qty {
            self.lock.release(guard)?;
            return Ok(false);
        }
        self.stock -= qty;
        self.lock.release(guard)?;
        Ok(true)
    }
}

fn run_tests() {
    let n1 = naive_increment_local().unwrap();
    let n2 = naive_increment_local().unwrap();
    assert_eq!(n2, n1 + 1);

    let mut store = MiniRedisLockStore::default();
    let mut dl = DistributedLock::new(&mut store, Duration::from_secs(30));

    let g1 = dl.try_acquire("order:pay", "instance-a:1").unwrap();
    assert!(g1.is_some());
    let g1 = g1.unwrap();

    // 第二个抢同一资源应失败（None）
    assert!(dl
        .try_acquire("order:pay", "instance-b:9")
        .unwrap()
        .is_none());

    // 错误 token 不能释放
    let g_fake = LockGuard {
        key: g1.key.clone(),
        token: "wrong".into(),
    };
    assert_eq!(
        dl.release(g_fake),
        Err(LockError::WrongOwner)
    );

    dl.release(g1.clone()).unwrap();
    assert!(dl.try_acquire("order:pay", "instance-b:9").unwrap().is_some());

    // 过期后可被他人获取（库里残留已过期的锁记录）
    let mut store2 = MiniRedisLockStore::default();
    store2.locks.insert(
        "lock:flash:sale".into(),
        LockRecord {
            owner: "pod-1".into(),
            expire_at: Instant::now() - Duration::from_secs(1),
        },
    );
    let mut dl2 = DistributedLock::new(&mut store2, Duration::from_secs(60));
    assert!(dl2
        .try_acquire("flash:sale", "pod-2")
        .unwrap()
        .is_some());

    // renew：持锁方延长 TTL，其他人仍抢不到
    let mut store3 = MiniRedisLockStore::default();
    let mut dl3 = DistributedLock::new(&mut store3, Duration::from_secs(10));
    let g3 = dl3.try_acquire("job", "worker-1").unwrap().unwrap();
    dl3.renew(&g3, Duration::from_secs(120)).unwrap();
    assert!(dl3.try_acquire("job", "worker-2").unwrap().is_none());
    dl3.release(g3).unwrap();

    // 库存示例
    let mut store4 = MiniRedisLockStore::default();
    let mut svc = InventoryService {
        stock: 2,
        lock: DistributedLock::new(&mut store4, Duration::from_secs(5)),
    };
    assert!(svc.try_deduct("sku-9", "buyer-a", 1).unwrap());
    assert!(svc.try_deduct("sku-9", "buyer-b", 1).unwrap());
    assert!(!svc.try_deduct("sku-9", "buyer-c", 1).unwrap());
    assert_eq!(svc.stock, 0);

    println!("✅ 分布式锁入门（内存模拟）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Distributed Lock ************\n");

    let mut store = MiniRedisLockStore::default();
    let mut dl = DistributedLock::new(&mut store, Duration::from_secs(15));

    println!("实例 A 抢锁 lock:daily-report ...");
    match dl.try_acquire("daily-report", "cron-pod-a") {
        Ok(Some(g)) => {
            println!("  ✅ 抢到，token={}", g.token);
            println!("  执行业务（本课只打印）");
            dl.release(g).unwrap();
            println!("  已释放锁");
        }
        Ok(None) => println!("  未抢到，别的实例在执行"),
        Err(e) => println!("  错误: {}", e),
    }

    println!(
        "\n本地 Mutex 计数: {}（只在本进程有效）",
        naive_increment_local().unwrap()
    );

    println!("\n对照 Redis: SET mylock $RANDOM NX EX 30 + Lua 释放");
    println!("https://redis.io/docs/latest/develop/use/patterns/distributed-locks/\n");

    run_tests();
}

fn main() {
    bootstrap();
}