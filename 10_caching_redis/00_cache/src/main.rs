/*
[主题]: Cache-Aside（旁路缓存）—— 先读缓存、未命中再回源、写库后删缓存
[难度]: Easy → Medium
[前置]: `09_persistence/00_repo`、`04_traits_generics`

一、主题说明
- **Cache-Aside** 是后端最常用的缓存模式：应用自己管「缓存」和「数据库」的读写顺序。
- 读：`GET cache` → 没有则查 DB → `SET cache`（可带 TTL 过期时间）。
- 写：先更新 DB，再 **删除** 缓存（或更新缓存；删缓存更简单、少并发坑）。
- 本课用 **内存假 DB + 内存假 Redis**，不连真 Redis，秒级自测。

二、写法对比
1. 直观版: 每次读都查「数据库」，没有缓存层 —— 简单但慢、DB 压力大。
2. 进阶版: `ProductService` 封装 Cache-Aside，统计 cache hit / miss，写操作 `invalidate`。

三、真实用途
- 商品详情、用户信息、配置项：Redis + PostgreSQL 经典组合。
- 生产 Rust 里常用 `redis` crate 的 `GET/SET/DEL`；TTL 用 `SETEX` 或 `EXPIRE`（见 `10_caching_redis/01_client`）。
- 注意：**缓存穿透/击穿/雪崩** 要额外策略（本课只打基础 Cache-Aside）。
资料: https://redis.io/docs/latest/develop/clients/rust/
      https://docs.rs/redis/latest/redis/
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

// ── 领域与错误 ───────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct Product {
    id: u64,
    name: String,
    price_cents: u64,
}

#[derive(Debug, PartialEq)]
enum AppError {
    NotFound(u64),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotFound(id) => write!(f, "商品 {} 不存在", id),
        }
    }
}

// ── 假数据库（权威数据源）────────────────────────────────────────

struct ProductDb {
    rows: HashMap<u64, Product>,
}

impl ProductDb {
    fn seed() -> Self {
        let mut rows = HashMap::new();
        rows.insert(
            1,
            Product {
                id: 1,
                name: "Rust 入门键盘".into(),
                price_cents: 19900,
            },
        );
        rows.insert(
            2,
            Product {
                id: 2,
                name: "Tokio 主题鼠标垫".into(),
                price_cents: 5900,
            },
        );
        Self { rows }
    }

    fn load_by_id(&self, id: u64) -> Result<Option<Product>, AppError> {
        Ok(self.rows.get(&id).cloned())
    }

    fn save(&mut self, product: Product) -> Result<(), AppError> {
        self.rows.insert(product.id, product);
        Ok(())
    }

    fn delete(&mut self, id: u64) -> Result<bool, AppError> {
        Ok(self.rows.remove(&id).is_some())
    }
}

// ── 缓存条目（带 TTL，模拟 Redis 过期）────────────────────────────

#[derive(Debug, Clone)]
struct CacheEntry {
    value: Product,
    expires_at: Instant,
}

impl CacheEntry {
    fn fresh(value: Product, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_alive(&self) -> bool {
        Instant::now() < self.expires_at
    }
}

/// 内存假 Redis：键 `product:{id}` → JSON 不必真序列化，直接存 Product
struct MemoryCache {
    ttl: Duration,
    store: HashMap<String, CacheEntry>,
}

impl MemoryCache {
    fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            store: HashMap::new(),
        }
    }

    fn key(id: u64) -> String {
        format!("product:{id}")
    }

    fn get(&mut self, id: u64) -> Option<Product> {
        let k = Self::key(id);
        match self.store.get(&k) {
            Some(entry) if entry.is_alive() => Some(entry.value.clone()),
            Some(_) => {
                self.store.remove(&k);
                None
            }
            None => None,
        }
    }

    fn set(&mut self, product: &Product) {
        let k = Self::key(product.id);
        self.store.insert(k, CacheEntry::fresh(product.clone(), self.ttl));
    }

    fn del(&mut self, id: u64) {
        self.store.remove(&Self::key(id));
    }
}

// ── 直观版：没有缓存 ─────────────────────────────────────────────

struct NaiveProductReader<'a> {
    db: &'a ProductDb,
}

impl<'a> NaiveProductReader<'a> {
    fn get(&self, id: u64) -> Result<Product, AppError> {
        match self.db.load_by_id(id)? {
            Some(p) => Ok(p),
            None => Err(AppError::NotFound(id)),
        }
    }
}

// ── 进阶版：Cache-Aside ──────────────────────────────────────────

#[derive(Debug, Default, Clone, Copy)]
struct CacheStats {
    hits: u32,
    misses: u32,
    db_reads: u32,
    invalidations: u32,
}

struct ProductService<'a> {
    db: &'a mut ProductDb,
    cache: &'a mut MemoryCache,
    stats: CacheStats,
}

impl<'a> ProductService<'a> {
  /// 读路径：Cache-Aside
    fn get_product(&mut self, id: u64) -> Result<Product, AppError> {
        if let Some(p) = self.cache.get(id) {
            self.stats.hits += 1;
            return Ok(p);
        }
        self.stats.misses += 1;
        self.stats.db_reads += 1;

        match self.db.load_by_id(id)? {
            Some(p) => {
                self.cache.set(&p);
                Ok(p)
            }
            None => Err(AppError::NotFound(id)),
        }
    }

    /// 写路径：先写 DB，再删缓存（避免旧数据长期留在 cache）
    fn update_product(&mut self, product: Product) -> Result<(), AppError> {
        if self.db.load_by_id(product.id)?.is_none() {
            return Err(AppError::NotFound(product.id));
        }
        self.db.save(product.clone())?;
        self.cache.del(product.id);
        self.stats.invalidations += 1;
        Ok(())
    }

    fn delete_product(&mut self, id: u64) -> Result<(), AppError> {
        if !self.db.delete(id)? {
            return Err(AppError::NotFound(id));
        }
        self.cache.del(id);
        self.stats.invalidations += 1;
        Ok(())
    }
}

fn run_tests() {
  // 直观版
    let db = ProductDb::seed();
    let naive = NaiveProductReader { db: &db };
    assert_eq!(naive.get(1).unwrap().name, "Rust 入门键盘");
    assert_eq!(naive.get(999), Err(AppError::NotFound(999)));

    // Cache-Aside：第一次 miss + 回源，第二次 hit
    let mut db2 = ProductDb::seed();
    let mut cache = MemoryCache::new(Duration::from_secs(300));
    let mut svc = ProductService {
        db: &mut db2,
        cache: &mut cache,
        stats: CacheStats::default(),
    };

    let p1 = svc.get_product(1).unwrap();
    assert_eq!(p1.price_cents, 19900);
    assert_eq!(svc.stats.misses, 1);
    assert_eq!(svc.stats.hits, 0);
    assert_eq!(svc.stats.db_reads, 1);

    let p1_again = svc.get_product(1).unwrap();
    assert_eq!(p1_again.name, p1.name);
    assert_eq!(svc.stats.hits, 1);
    assert_eq!(svc.stats.db_reads, 1);

    // 更新后缓存失效，再读应 miss 一次并拿到新价
    svc.update_product(Product {
        id: 1,
        name: "Rust 机械键盘 Pro".into(),
        price_cents: 29900,
    })
    .unwrap();
    assert_eq!(svc.stats.invalidations, 1);

    // 更新后缓存已删，再读会先 miss 再回源
    let before_hits = svc.stats.hits;
    let fresh = svc.get_product(1).unwrap();
    assert_eq!(fresh.price_cents, 29900);
    assert_eq!(fresh.name, "Rust 机械键盘 Pro");
    assert_eq!(svc.stats.misses, 2);
    assert_eq!(svc.stats.hits, before_hits);

    // TTL 过期：把过期时间设为过去
    let mut cache2 = MemoryCache::new(Duration::from_secs(1));
    cache2.store.insert(
        MemoryCache::key(2),
        CacheEntry {
            value: Product {
                id: 2,
                name: "过期商品".into(),
                price_cents: 0,
            },
            expires_at: Instant::now() - Duration::from_secs(1),
        },
    );
    assert!(cache2.get(2).is_none());

    let mut db3 = ProductDb::seed();
    let mut cache3 = MemoryCache::new(Duration::from_secs(60));
    let mut svc2 = ProductService {
        db: &mut db3,
        cache: &mut cache3,
        stats: CacheStats::default(),
    };
    svc2.delete_product(2).unwrap();
    assert_eq!(svc2.get_product(2), Err(AppError::NotFound(2)));

    println!("✅ Cache-Aside 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Cache-Aside ************\n");

    let mut db = ProductDb::seed();
    let mut cache = MemoryCache::new(Duration::from_secs(120));
    let mut svc = ProductService {
        db: &mut db,
        cache: &mut cache,
        stats: CacheStats::default(),
    };

    println!("第一次读 id=1（预期 cache miss，回源 DB）");
    match svc.get_product(1) {
        Ok(p) => println!("  → {:?}", p),
        Err(e) => println!("  → 错误: {}", e),
    }
    println!(
        "  stats: hits={} misses={} db_reads={}",
        svc.stats.hits, svc.stats.misses, svc.stats.db_reads
    );

    println!("\n第二次读 id=1（预期 cache hit）");
    let _ = svc.get_product(1);
    println!(
        "  stats: hits={} misses={} db_reads={}",
        svc.stats.hits, svc.stats.misses, svc.stats.db_reads
    );

    let naive_db = ProductDb::seed();
    let naive = NaiveProductReader { db: &naive_db };
    println!(
        "\n直观版无缓存，每次读都碰 DB: {:?}",
        naive.get(2).map(|p| p.name)
    );

    println!("\n对照生产: cache 换成 redis::Commands::get/set/del，DB 换成 sqlx。");
    println!("Redis Rust 客户端: https://redis.io/docs/latest/develop/clients/rust/\n");

    run_tests();
}

fn main() {
    bootstrap();
}