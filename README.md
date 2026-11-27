# krs-project

Rust **从零到顶尖精通**的个人学习仓库。前提：你是 **Rust 小白**（可以有其他语言基础，但 Rust 从语法与所有权讲起）。

姊妹项目（同级目录）：`kjv-project`（Java）、`kgo-project`（Go）、`kcs-project`（C#）、`kpy_project`（Python）、`kts-project`（TypeScript）。编号骨架可对照，但 **Rust 请严格按本仓库阶段顺序**；精通线顺序按 Rust 特性定制（ownership → async → unsafe → 性能）。

> 说明：阶段目录用 `NN_slug` 方便 Cursor 全自动生成与进度管理，**不是**死板照搬别的语言课表；课表可按推进增补，但学习顺序不要乱跳。Rust 比「语法周」多一门硬课：**所有权/借用**，请勿跳过。

## 目录（规划 / 已生成）

| 路径 | 阶段 | 内容 | 状态 |
| --- | --- | --- | --- |
| `00_rust_basics/` | 阶段 1 | Rust 语法入门 | ✅ 已生成 |
| `01_ownership_borrowing/` | 阶段 2 | 所有权与借用 | ✅ 已生成 |
| `02_structs_enums/` | 阶段 2 | 结构体与枚举 | ✅ 已生成 |
| `03_error_collections/` | 阶段 2 | 错误处理与集合 | ✅ 已生成 |
| `04_traits_generics/` | 阶段 3 | Trait 与泛型 | ✅ 已生成 |
| `05_cargo_modules/` | 阶段 3 | Cargo / 模块 / workspace | ✅ 已生成 |
| `06_std_concurrency/` | 阶段 4 | 标准库并发 | ✅ 已生成 |
| `07_async_tokio/` | 阶段 4 | 异步与 Tokio | ✅ 已生成 |
| `08_web_axum/` | 阶段 5 | Axum Web API | ✅ 已生成 |
| `09_persistence/` | 阶段 5 | 持久化（sqlx 心智） | ✅ 已生成 |
| `10_caching_redis/` | 阶段 6 | 缓存与 Redis | ✅ 已生成 |
| `11_messaging/` | 阶段 6 | 消息与异步解耦 | ✅ 已生成 |
| `12_grpc_tonic/` | 阶段 7 | 微服务与 tonic | ✅ 已生成 |
| `13_testing/` | 穿插 | 测试与基准 | ✅ 已生成 |
| `14_observability/` | 阶段 7 | 可观测性 | ✅ 已生成 |
| `15_project_capstone/` | 毕业 | 综合小项目 | ✅ 已生成 |
| `16_production_rust/` | 精通 | 生产级 Rust 工程 | ✅ 已生成 |
| `17_async_advanced/` | 精通 | 异步深挖（Pin/Future） | ✅ 已生成 |
| `18_unsafe_ffi/` | 精通 | Unsafe 与 FFI | ✅ 已生成 |
| `19_perf_memory/` | 精通 | 性能与内存 | ✅ 已生成 |
| `20_cloud_native/` | 精通 | 云原生 | ✅ |
| `21_architecture/` | 精通 | 架构与 SRE | ✅ 已生成 |

## 环境

- **Rust 最新稳定版**（通过 rustup 安装）。
- 安装：https://www.rust-lang.org/tools/install
- 学习入口：https://www.rust-lang.org/learn
- 验证种子课：

```bash
cd krs-project
chmod +x scripts/verify_modules.sh
./scripts/verify_modules.sh 00_rust_basics
```

## 一、Rust 小白到精通：还该学什么（推荐度从高到低）

下表按**推荐度**从高到低；**目录编号按学习顺序**。推荐度 ≠ 学习顺序（例如 Axum 很值钱，但仍必须先学所有权与 async）。

判定：必须/强相关就业或生产；可以系统学；有必要（投入产出高于「只刷语法题」）。

**外链一律明文完整 URL（已 HTTP 核验），不要只看锚文字。**

| 排名 | 推荐度 | 主题 / 代表资料（明文 URL） | 目录 | 核心内容 | 为什么有必要 |
| --- | --- | --- | --- | --- | --- |
| 1 | ⭐⭐⭐⭐⭐+ | 所有权：https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html ；The Book：https://doc.rust-lang.org/book/ ；RBE：https://doc.rust-lang.org/rust-by-example/ | `01_ownership_borrowing` | move、借用、切片、生命周期直觉 | **Rust 灵魂**；不会这个，编译器天天骂你，后面全是玄学。 |
| 2 | ⭐⭐⭐⭐⭐+ | Axum：https://docs.rs/axum/latest/axum/ ；仓库：https://github.com/tokio-rs/axum ；Tower：https://docs.rs/tower/latest/tower/ | `08_web_axum` | Router、Extractor、中间件、分层 | **Rust 后端岗常见基本盘**；不要零基础直接跳这里。 |
| 3 | ⭐⭐⭐⭐⭐ | 官方入门：https://www.rust-lang.org/learn ；安装：https://www.rust-lang.org/tools/install ；The Book：https://doc.rust-lang.org/stable/book/ ；Rustlings：https://rustlings.rust-lang.org/ | `00_rust_basics` | 类型、控制流、cargo | **地基**。配合 Rustlings 手感最好。 |
| 4 | ⭐⭐⭐⭐⭐ | Tokio 教程：https://tokio.rs/tokio/tutorial ；Async Book：https://rust-lang.github.io/async-book/ ；Book 异步章：https://doc.rust-lang.org/stable/book/ch17-00-async-await.html | `07_async_tokio` | async/await、runtime | 现代 Rust 服务几乎处处异步；面试与性能分水岭。 |
| 5 | ⭐⭐⭐⭐⭐ | sqlx：https://docs.rs/sqlx/latest/sqlx/ ；仓库：https://github.com/launchbadge/sqlx ；Serde：https://serde.rs/ | `09_persistence` | 查询、事务心智 | 业务系统几乎都落库。 |
| 6 | ⭐⭐⭐⭐☆ | Trait/泛型：https://doc.rust-lang.org/book/ch10-00-generics.html ；API Guidelines：https://rust-lang.github.io/api-guidelines/ | `04_traits_generics` | trait、泛型、dyn | 写出可扩展、可测试的抽象。 |
| 7 | ⭐⭐⭐⭐☆ | 结构体/枚举：https://doc.rust-lang.org/book/ ；https://doc.rust-lang.org/rust-by-example/ | `02_structs_enums` | struct、enum、match、Option | 日常建模语言。 |
| 8 | ⭐⭐⭐⭐☆ | 错误与集合：https://doc.rust-lang.org/std/ ；https://doc.rust-lang.org/book/ | `03_error_collections` | Result、Vec、HashMap | 错误处理是 Rust 风格核心。 |
| 9 | ⭐⭐⭐⭐☆ | Cargo：https://doc.rust-lang.org/cargo/ ；Guide：https://doc.rust-lang.org/cargo/guide/ ；Workspace：https://doc.rust-lang.org/cargo/reference/workspaces.html ；docs.rs：https://docs.rs/ | `05_cargo_modules` | 模块、features、workspace | 现代 Rust 工程日常。 |
| 10 | ⭐⭐⭐⭐☆ | 标准库并发：https://doc.rust-lang.org/book/ch16-00-concurrency.html ；Mutex：https://doc.rust-lang.org/std/sync/struct.Mutex.html ；Arc：https://doc.rust-lang.org/std/sync/struct.Arc.html | `06_std_concurrency` | thread、channel、锁 | 先懂 std，再上 async，排障更稳。 |
| 11 | ⭐⭐⭐⭐ | Redis：https://redis.io/docs/latest/develop/clients/rust/ ；https://docs.rs/redis/latest/redis/ | `10_caching_redis` | 缓存、过期、锁入门 | 性能与会话/秒杀标配。 |
| 12 | ⭐⭐⭐⭐ | 测试：https://doc.rust-lang.org/book/ch11-00-testing.html ；https://doc.rust-lang.org/rust-by-example/testing.html ；Criterion：https://docs.rs/criterion/latest/criterion/ | `13_testing` | 表驱动、集成、基准 | 没有测试谈不上精通与协作。 |
| 13 | ⭐⭐⭐⭐ | 可观测：https://docs.rs/tracing/latest/tracing/ ；https://github.com/tokio-rs/tracing ；OTel：https://opentelemetry.io/docs/languages/rust/ | `14_observability` | tracing、指标、追踪 | 线上排障与 SRE 基本功。 |
| 14 | ⭐⭐⭐☆ | tonic：https://docs.rs/tonic/latest/tonic/ ；https://github.com/hyperium/tonic | `12_grpc_tonic` | gRPC、服务拆分 | 微服务与内部 RPC 常见。 |
| 15 | ⭐⭐⭐☆ | 消息：https://nats.io/docs/ ；https://docs.rs/rdkafka/latest/rdkafka/ | `11_messaging` | 异步、幂等 | 业务复杂后必遇。 |
| 16 | ⭐⭐⭐ | 综合项目 + 书：https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590 ；https://www.manning.com/books/rust-servers-services-and-apps ；https://nostarch.com/rust-rustaceans | `15_project_capstone` | 鉴权+CRUD+缓存 | **作品集**；书作系统补强。 |
| 17 | ⭐⭐⭐ | 语言参考 / 源码：https://doc.rust-lang.org/reference/ ；https://doc.rust-lang.org/std/ ；https://github.com/rust-lang/rust ；https://blog.rust-lang.org/ | 穿插查阅 | 规范、标准库、源码 | 精通期对照用。 |
| 18 | ⭐⭐ | 视频：https://www.youtube.com/@RustVideos ；练习：https://rustlings.rust-lang.org/ ；https://github.com/rust-lang/rustlings | 穿插 | 官方/社区演讲与练习 | 辅助理解；主线仍以手敲课为准。 |
| 19 | ⭐⭐ | 过早追 unsafe 炫技 / 无基础直接上内核或复杂 proc-macro | 不单独优先 | — | 机会成本高；主线完成前不要分心。 |

### 各知识点「资料包」速查（阶段内还会再细化）

| 知识点簇 | 明文资料 URL |
| --- | --- |
| 安装与第一课 | https://www.rust-lang.org/tools/install ；https://www.rust-lang.org/learn ；https://doc.rust-lang.org/book/ |
| 练习与例子 | https://doc.rust-lang.org/rust-by-example/ ；https://rustlings.rust-lang.org/ ；https://github.com/rust-lang/rustlings |
| 所有权 | https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html |
| 异步 | https://doc.rust-lang.org/stable/book/ch17-00-async-await.html ；https://rust-lang.github.io/async-book/ ；https://tokio.rs/tokio/tutorial |
| Web | https://docs.rs/axum/latest/axum/ ；https://docs.rs/tower/latest/tower/ |
| 数据库 | https://docs.rs/sqlx/latest/sqlx/ ；https://serde.rs/ |
| Redis | https://redis.io/docs/latest/develop/clients/rust/ ；https://docs.rs/redis/latest/redis/ |
| 测试 | https://doc.rust-lang.org/book/ch11-00-testing.html ；https://docs.rs/criterion/latest/criterion/ |
| 诊断与监控 | https://docs.rs/tracing/latest/tracing/ ；https://opentelemetry.io/docs/languages/rust/ |
| Unsafe / Nomicon | https://doc.rust-lang.org/nomicon/ ；https://doc.rust-lang.org/reference/ |
| 性能 | https://docs.rs/criterion/latest/criterion/ ；https://doc.rust-lang.org/rustc/profile-guided-optimization.html |
| 经典书 | https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590 ；https://nostarch.com/rust-rustaceans ；https://www.amazon.com/s?k=Rust+for+Rustaceans ；https://www.manning.com/books/rust-in-action ；https://www.manning.com/books/code-like-a-pro-in-rust |
| 生产错误处理 | https://docs.rs/thiserror/latest/thiserror/ ；https://docs.rs/anyhow/latest/anyhow/ ；https://rust-lang.github.io/api-guidelines/ |
| SRE | https://sre.google/books/ |
| 官方视频 | https://www.youtube.com/@RustVideos |

## 二、阶段路线（学习顺序）

| 阶段 | 建议周期 | 目录 | 做什么 | 验收标准 |
| --- | --- | --- | --- | --- |
| 1. 语法 | 1～2 周 | `00_rust_basics` | 每天 1～2 课，手敲 + verify；可并行 Rustlings | 能独立写小程序 |
| 2. 所有权与类型 | 3～5 周 | `01_`、`02_`、`03_` | ownership + struct/enum + Result/集合 | 能解释 move/借用；会用 Option/Result |
| 3. 抽象与工程 | 2～3 周 | `04_`、`05_`、`13_`（开始） | trait/泛型 + Cargo + 测试 | 会写 trait 对象；会 `cargo test` |
| 4. 并发与异步 | 3～5 周 | `06_`、`07_` | std 并发 + Tokio | 能写异步服务骨架；懂勿在 async 里阻塞 |
| 5. Web 与存储 | 1～2 月 | `08_`、`09_` | Axum + sqlx | 独立完成带校验的 REST + DB |
| 6. 缓存与消息 | 2～4 周 | `10_`、`11_` | 接到上阶段项目 | 会缓存查询；理解异步与幂等 |
| 7. 微服务与观测 | 1～2 月 | `12_`、`14_`、`15_` | tonic 心智 + 可观测 + 毕业项目 | 作品集可演示 |
| 8. 顶尖精通 | 持续 | `16→17→18→19→20→21`（Rust 专用） | 生产 → async 深挖 → unsafe/FFI → 性能 → 云原生 → 架构 | 各方向能讲清原理并对照真实组件 |

### 为何精通线是 `16→17→18→19→20→21`？

| 顺序 | 目录 | 为何排在这里 |
| --- | --- | --- |
| 1 | `16_production_rust` | 先能稳定上线（错误类型、配置、弹性、Clippy） |
| 2 | `17_async_advanced` | Tokio 用熟后再看 Pin/Future，体感更强 |
| 3 | `18_unsafe_ffi` | 大量安全代码之后再碰 unsafe，避免「未学先伤」 |
| 4 | `19_perf_memory` | 有真实异步/服务代码后再做剖析更有意义 |
| 5 | `20_cloud_native` | 容器与部署协同 |
| 6 | `21_architecture` | 架构与 SRE 收官 |

## 三、用 Cursor 全自动生成（rules + skill）

| 类型 | 路径 | 作用 |
| --- | --- | --- |
| Rule | `.cursor/rules/krs-learning-standards.mdc` | 编号、Cargo 课包、小白友好、verify、明文链接、顶尖精通终点 |
| Skill | `.cursor/skills/rust-study-generator/SKILL.md` | 按阶段生成课、模板、分批提交 |
| 课表 | `.cursor/skills/rust-study-generator/references/roadmap.md` | 各阶段课表明细与精通顺序 |
| 资料包 | `.cursor/skills/rust-study-generator/references/resources.md` | 已 HTTP 核验的明文 URL |

**用法：** Cursor 打开 **`krs-project` 根目录**，粘贴下面「全自动 Prompt」。

### 推荐粘贴的「全自动」Prompt（复制整段）

```text
请严格按 rust-study-generator skill 与 .cursor/rules/krs-learning-standards.mdc 执行。

我的起点：Rust 小白（可能有一点其他语言基础，但请按零基础讲 Rust）。
工具链：使用 rustup 最新稳定 rustc/cargo；生成的代码与讲解按当前稳定版与 edition，不要用过时写法。
目标：按 README「阶段路线」顺序，把尚未生成的目录全部自动生成完，使我按顺序学完后达到可就业的 Rust 水平，并继续精通各方向（ownership/类型扎实、会 Tokio + Axum + sqlx、会 Redis/消息入门、会测试与可观测、有综合小项目，再完成 16→21）。

生成顺序（跳过已完成的课，补齐缺课）：
00_rust_basics →
01_ownership_borrowing →
02_structs_enums →
03_error_collections →
04_traits_generics →
05_cargo_modules →
06_std_concurrency →
07_async_tokio →
08_web_axum →
09_persistence →
10_caching_redis →
11_messaging →
12_grpc_tonic →
13_testing →
14_observability →
15_project_capstone →
（精通线）
16_production_rust →
17_async_advanced →
18_unsafe_ffi →
19_perf_memory →
20_cloud_native →
21_architecture

硬性要求：
1. 一次只做一个 NN_xxx；写完更新该阶段 README 与根 README「已生成」表。
2. 每课：Cargo.toml + src/main.rs、中文注释、直观版+进阶版（有价值时）、run_tests()+bootstrap()+main、可用 ./scripts/verify_modules.sh 跑通。
3. 每阶段 verify 全绿后单独 git commit；全部完成后 git push。
4. 内容必须由浅入深，禁止默认我已会 ownership / Tokio / Axum / unsafe。
5. 不要无关重构；不要 force push。
6. README 外链必须明文完整 URL，禁止未验证假链接。

现在从下一个未完成的阶段开始全自动执行，直到全部完成。
```

### 精通线追加 Prompt

```text
请帮我全自动推进 16 → 17 → 18 → 19 → 20 → 21，保证全面，细节完美。
（这是 Rust 专用精通顺序：生产 → async 深挖 → unsafe/FFI → 性能 → 云原生 → 架构。）
严格按 rust-study-generator 与 krs-learning-standards：每阶段 verify 全绿后单独 commit，全部完成再 push，不要 force push。
```

## 四、和姊妹仓的关系

- `kjv-project`：Java 后端
- `kgo-project`：Go 后端
- `kcs-project`：C# / .NET 后端
- `krs-project`：Rust（本仓库）
- `kpy_project`：Python / AI
- `kts-project`：TypeScript / 前端

不要一上来只抄 Axum 教程；请按本 README 阶段顺序推进，尤其把 **所有权** 学扎实。
