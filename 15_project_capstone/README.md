# `15_project_capstone` | 毕业综合项目

- **目标**: 用**一块可运行、可自测**的迷你 API 串起就业主线 —— **鉴权（Bearer）+ User/Item CRUD（Repository）+ Cache-Aside 读缓存**；全程内存假实现，**不 bind 真端口、不连 DB/Redis**，对照真实 Axum + sqlx + Redis 项目结构。
- **前置**: `08_web_axum`（Router / 中间件 / 分层）、`09_persistence`（Repository trait）、`10_caching_redis`（Cache-Aside）；建议已学 `04_traits_generics`
- **环境**: rustup 最新稳定版；验证：`./scripts/verify_modules.sh 15_project_capstone`
- **推荐资料（明文 URL，已核验）**:
  - https://docs.rs/axum/latest/axum/
  - https://docs.rs/sqlx/latest/sqlx/
  - https://docs.rs/tracing/latest/tracing/
  - https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590
  - https://www.manning.com/books/rust-servers-services-and-apps
  - https://nostarch.com/rust-rustaceans

## 章节进度

| 编号 | 目录 | 包名 | 核心点 | 状态 |
| --- | --- | --- | --- | --- |
| `00_` | `00_app/` | `l00_mini_api_capstone` | Auth + User/Item CRUD + Cache-Aside + MiniRouter | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 说明 | 明文 URL |
| --- | --- | --- |
| Axum Router / Handler | HTTP 路由与提取器心智 | https://docs.rs/axum/latest/axum/ |
| sqlx 持久化 | 异步 SQL + 类型映射 | https://docs.rs/sqlx/latest/sqlx/ |
| tracing 可观测 | 结构化日志与 span | https://docs.rs/tracing/latest/tracing/ |
| Programming Rust | 系统向全书补强 | https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590 |
| Rust Servers, Services and Apps | 服务端实战书 | https://www.manning.com/books/rust-servers-services-and-apps |
| Rust for Rustaceans | 进阶 idioms | https://nostarch.com/rust-rustaceans |

## 与真实栈的对应关系

| 本课 (`00_app`) | 生产 Rust 后端 |
| --- | --- |
| `AuthService::verify_bearer` | JWT / API Key + Tower Layer 或 Axum middleware |
| `UserRepository` / `ItemRepository` + `InMemory*` | sqlx `Pool` + `Pg*Repo` |
| `ItemService::get_cached` + `ItemCache` | Redis `GET` → miss → sqlx → `SETEX` |
| `NaiveCapstone::dispatch` | 反模式：handler 里直接摸 DB（对照用） |
| `MiniRouter::handle` | `axum::Router::call` / 方法路由表 |
| `bootstrap` 里 `[trace]` 行 | `tracing::info!(...)`（见 `14_observability`） |

## 路由一览（内存模拟）

| Method | Path | 鉴权 | 说明 |
| --- | --- | --- | --- |
| GET | `/health` | 否 | 存活探测 |
| POST | `/users` | Bearer | body: `email=...` |
| GET | `/users/{id}` | Bearer | 查用户 |
| DELETE | `/users/{id}` | Bearer | 删用户 |
| POST | `/items` | Bearer | body: `owner_id=&title=` |
| GET | `/items/{id}` | Bearer | **Cache-Aside** 读 |
| PUT | `/items/{id}` | Bearer | 更新后 invalidate 缓存 |
| DELETE | `/items/{id}` | Bearer | 删库 + 删缓存 |

演示用 Token：`Authorization: Bearer capstone-secret`

## 运行单课

```bash
cargo run --manifest-path 15_project_capstone/00_app/Cargo.toml
```

## 作品集提示

1. 把本仓库课 + 你自己的 Axum 真项目 README 链在一起，写明「迷你课证明我懂分层；真项目证明我能上线」。
2. 换真栈时优先动 **Repository impl** 与 **Cache 客户端**，Router/Service 接口尽量不动 —— 这就是本课要练的边界。
3. 精通线下一阶段 `16_production_rust` 会补错误模型、配置、弹性与 Clippy 指南。

**本目录总课数：1（已完成 1）**