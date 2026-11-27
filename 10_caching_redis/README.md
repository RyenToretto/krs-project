# `10_caching_redis` | 缓存与 Redis

- **目标**: Cache-Aside 读写顺序、Redis 客户端命令/连接池心智、分布式锁 `SET NX EX` + token 释放（全程 **trait + 内存假实现**，**不连真 Redis**）
- **前置**: `09_persistence`（Repository / 回源）；建议已学 `04_traits_generics`、`06_std_concurrency`
- **环境**: rustup 最新稳定版；验证：`./scripts/verify_modules.sh 10_caching_redis`
- **推荐资料（明文 URL，已核验）**:
  - https://redis.io/docs/latest/develop/clients/rust/
  - https://docs.rs/redis/latest/redis/
  - https://redis.io/docs/latest/develop/use/patterns/distributed-locks/
  - https://www.youtube.com/@RustVideos

## 章节进度

| 编号 | 目录 | 包名 | 核心点 | 状态 |
| --- | --- | --- | --- | --- |
| `00_` | `00_cache/` | `l00_cache_aside` | Cache-Aside：读穿、写后删缓存、TTL、命中率统计 | ✅ |
| `01_` | `01_client/` | `l00_redis_client_mental` | 假 Redis：`GET/SET/SETEX/DEL`、连接池借还 | ✅ |
| `02_` | `02_lock/` | `l00_distributed_lock_intro` | `SET NX EX`、owner token、续约与 Lua 心智释放 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 说明 | 明文 URL |
| --- | --- | --- |
| Redis Rust 客户端指南 | 官方推荐接入方式 | https://redis.io/docs/latest/develop/clients/rust/ |
| redis crate API | `Client`、`Connection`、命令 | https://docs.rs/redis/latest/redis/ |
| 分布式锁模式 | 单实例 SET NX + token | https://redis.io/docs/latest/develop/use/patterns/distributed-locks/ |
| Rust 视频 | 补充听/看 | https://www.youtube.com/@RustVideos |

## 与真实 Redis / redis crate 的对应关系

| 本课概念 | 生产里大致是什么 |
| --- | --- |
| `MemoryCache` + `ProductService` | `redis::Commands::get/set/del` + sqlx 回源 |
| `RedisNode::checkout_slot` / `checkin_slot` | `Client::open` + 池 / `ConnectionManager` |
| `RedisStore::setex` | `SET key value EX seconds` |
| `MiniRedisLockStore::set_nx_ex` | `SET lock_key token NX EX 30` |
| `release_if_owner` | Lua `GET` 比较后 `DEL` |

## 运行单课

```bash
cargo run --manifest-path 10_caching_redis/00_cache/Cargo.toml
cargo run --manifest-path 10_caching_redis/01_client/Cargo.toml
cargo run --manifest-path 10_caching_redis/02_lock/Cargo.toml
```

**本目录总课数：3（已完成 3）**