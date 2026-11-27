# 已核验资料包（明文 URL）

> 规则：写入 README / 课内注释的外链必须是本表或同次 HTTP 核验通过的完整 URL；禁止 Markdown 仅锚文字、禁止未核验假链接。  
> 核验方式：`curl -sL -o /dev/null -w "%{http_code}" -A "Mozilla/5.0" <url>`，目标优先 **200**。  
> 更新日：2026-11-27；建议通过 rustup 安装最新稳定 Rust。  
> 说明：`crates.io` 对部分自动化客户端返回非 200，crate 文档统一用 **docs.rs**；crate 检索用 Cargo Book / docs.rs。

## 总览

| 用途 | 明文 URL |
| --- | --- |
| 学习入口 | https://www.rust-lang.org/learn |
| 安装 | https://www.rust-lang.org/tools/install |
| The Book | https://doc.rust-lang.org/book/ |
| The Book (stable) | https://doc.rust-lang.org/stable/book/ |
| Rust By Example | https://doc.rust-lang.org/rust-by-example/ |
| Cargo Book | https://doc.rust-lang.org/cargo/ |
| Cargo Guide | https://doc.rust-lang.org/cargo/guide/ |
| std | https://doc.rust-lang.org/std/ |
| Reference | https://doc.rust-lang.org/reference/ |
| Edition Guide | https://doc.rust-lang.org/edition-guide/ |
| Clippy | https://doc.rust-lang.org/clippy/ |
| Clippy lint 列表 | https://rust-lang.github.io/rust-clippy/master/ |
| Rustlings | https://rustlings.rust-lang.org/ |
| Rustlings 仓库 | https://github.com/rust-lang/rustlings |
| Async Book | https://rust-lang.github.io/async-book/ |
| Book 异步章 | https://doc.rust-lang.org/stable/book/ch17-00-async-await.html |
| Tokio | https://tokio.rs/ |
| Tokio Tutorial | https://tokio.rs/tokio/tutorial |
| Tokio docs | https://docs.rs/tokio/latest/tokio/ |
| Axum docs | https://docs.rs/axum/latest/axum/ |
| Axum 仓库 | https://github.com/tokio-rs/axum |
| Tower | https://docs.rs/tower/latest/tower/ |
| sqlx | https://docs.rs/sqlx/latest/sqlx/ |
| sqlx 仓库 | https://github.com/launchbadge/sqlx |
| Redis Rust 客户端文档 | https://redis.io/docs/latest/develop/clients/rust/ |
| redis crate | https://docs.rs/redis/latest/redis/ |
| tonic | https://docs.rs/tonic/latest/tonic/ |
| tonic 仓库 | https://github.com/hyperium/tonic |
| tracing | https://docs.rs/tracing/latest/tracing/ |
| tracing 仓库 | https://github.com/tokio-rs/tracing |
| OTel Rust | https://opentelemetry.io/docs/languages/rust/ |
| 测试章 | https://doc.rust-lang.org/book/ch11-00-testing.html |
| RBE 测试 | https://doc.rust-lang.org/rust-by-example/testing.html |
| docs.rs | https://docs.rs/ |
| YouTube Rust | https://www.youtube.com/@RustVideos |
| Nomicon | https://doc.rust-lang.org/nomicon/ |
| Embedded Book | https://doc.rust-lang.org/embedded-book/ |
| Embedded 入口 | https://www.rust-lang.org/what/embedded |
| API Guidelines | https://rust-lang.github.io/api-guidelines/ |
| rustc PGO | https://doc.rust-lang.org/rustc/profile-guided-optimization.html |
| Criterion | https://docs.rs/criterion/latest/criterion/ |
| Programming Rust（书） | https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590 |
| Rust for Rustaceans（出版社） | https://nostarch.com/rust-rustaceans |
| Rust for Rustaceans（检索） | https://www.amazon.com/s?k=Rust+for+Rustaceans |
| Rust in Action | https://www.manning.com/books/rust-in-action |
| Rust Servers Services Apps | https://www.manning.com/books/rust-servers-services-and-apps |
| Code Like a Pro in Rust | https://www.manning.com/books/code-like-a-pro-in-rust |
| SRE 书籍 | https://sre.google/books/ |
| rust 源码 | https://github.com/rust-lang/rust |
| 博客 | https://blog.rust-lang.org/ |
| Serde | https://serde.rs/ |
| thiserror | https://docs.rs/thiserror/latest/thiserror/ |
| anyhow | https://docs.rs/anyhow/latest/anyhow/ |
| Pin | https://doc.rust-lang.org/std/pin/struct.Pin.html |
| Workspaces | https://doc.rust-lang.org/cargo/reference/workspaces.html |
| NATS | https://nats.io/docs/ |
| rdkafka | https://docs.rs/rdkafka/latest/rdkafka/ |

## 按阶段

### `00_rust_basics`
https://www.rust-lang.org/tools/install ；https://doc.rust-lang.org/book/ ；https://doc.rust-lang.org/rust-by-example/ ；https://rustlings.rust-lang.org/

### `01_ownership_borrowing`
https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html ；https://doc.rust-lang.org/book/ ；https://doc.rust-lang.org/rust-by-example/

### `02_structs_enums`
https://doc.rust-lang.org/book/ ；https://doc.rust-lang.org/rust-by-example/

### `03_error_collections`
https://doc.rust-lang.org/book/ ；https://doc.rust-lang.org/std/ ；https://doc.rust-lang.org/rust-by-example/

### `04_traits_generics`
https://doc.rust-lang.org/book/ch10-00-generics.html ；https://rust-lang.github.io/api-guidelines/

### `05_cargo_modules`
https://doc.rust-lang.org/cargo/ ；https://doc.rust-lang.org/cargo/guide/ ；https://doc.rust-lang.org/cargo/reference/workspaces.html ；https://docs.rs/

### `06_std_concurrency`
https://doc.rust-lang.org/book/ch16-00-concurrency.html ；https://doc.rust-lang.org/std/sync/struct.Mutex.html ；https://doc.rust-lang.org/std/sync/struct.Arc.html

### `07_async_tokio`
https://doc.rust-lang.org/stable/book/ch17-00-async-await.html ；https://rust-lang.github.io/async-book/ ；https://tokio.rs/tokio/tutorial ；https://docs.rs/tokio/latest/tokio/

### `08_web_axum`
https://docs.rs/axum/latest/axum/ ；https://github.com/tokio-rs/axum ；https://docs.rs/tower/latest/tower/

### `09_persistence`
https://docs.rs/sqlx/latest/sqlx/ ；https://github.com/launchbadge/sqlx ；https://serde.rs/

### `10_caching_redis`
https://redis.io/docs/latest/develop/clients/rust/ ；https://docs.rs/redis/latest/redis/

### `11_messaging`
https://nats.io/docs/ ；https://docs.rs/rdkafka/latest/rdkafka/

### `12_grpc_tonic`
https://docs.rs/tonic/latest/tonic/ ；https://github.com/hyperium/tonic

### `13_testing`
https://doc.rust-lang.org/book/ch11-00-testing.html ；https://doc.rust-lang.org/rust-by-example/testing.html ；https://docs.rs/criterion/latest/criterion/

### `14_observability`
https://docs.rs/tracing/latest/tracing/ ；https://github.com/tokio-rs/tracing ；https://opentelemetry.io/docs/languages/rust/

### `15_project_capstone`
https://docs.rs/axum/latest/axum/ ；https://docs.rs/sqlx/latest/sqlx/ ；https://docs.rs/tracing/latest/tracing/

### `16_production_rust`
https://docs.rs/thiserror/latest/thiserror/ ；https://docs.rs/anyhow/latest/anyhow/ ；https://rust-lang.github.io/api-guidelines/ ；https://doc.rust-lang.org/clippy/

### `17_async_advanced`
https://rust-lang.github.io/async-book/ ；https://doc.rust-lang.org/std/pin/struct.Pin.html ；https://docs.rs/tokio/latest/tokio/

### `18_unsafe_ffi`
https://doc.rust-lang.org/nomicon/ ；https://doc.rust-lang.org/reference/

### `19_perf_memory`
https://doc.rust-lang.org/rustc/profile-guided-optimization.html ；https://docs.rs/criterion/latest/criterion/ ；https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590

### `20_cloud_native`
https://doc.rust-lang.org/embedded-book/ ；https://www.rust-lang.org/what/embedded ；https://docs.rs/tracing/latest/tracing/

### `21_architecture`
https://rust-lang.github.io/api-guidelines/ ；https://sre.google/books/ ；https://nostarch.com/rust-rustaceans
