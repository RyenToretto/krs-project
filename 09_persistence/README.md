# `09_persistence` | 持久化

- **目标**: Repository trait、sqlx 查询/绑定心智、事务 commit/rollback 心智（全程 trait + 内存假实现，**不连真库**）
- **前置**: `08_web_axum`（分层）；建议已学 `04_traits_generics`、`03_error_collections`
- **环境**: rustup 最新稳定版；验证：`./scripts/verify_modules.sh 09_persistence`
- **推荐资料（明文 URL，已核验）**:
  - https://docs.rs/sqlx/latest/sqlx/
  - https://github.com/launchbadge/sqlx
  - https://serde.rs/
  - https://docs.rs/sqlx/latest/sqlx/trait.Executor.html
  - https://www.youtube.com/@RustVideos

## 章节进度

| 编号 | 目录 | 包名 | 核心点 | 状态 |
| --- | --- | --- | --- | --- |
| `00_` | `00_repo/` | `l00_repo_trait` | Repository trait + `InMemoryUserRepo`，Service 只依赖 trait | ✅ |
| `01_` | `01_sqlx/` | `l00_sqlx_mental` | 假 SQL 执行器、`?` 参数绑定、防拼接注入直觉 | ✅ |
| `02_` | `02_tx/` | `l00_transactions_mental` | 内存事务草稿、`begin` / `commit` / `rollback`、转账示例 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 说明 | 明文 URL |
| --- | --- | --- |
| sqlx 总览 | async Rust SQL 工具包、Pool、驱动 | https://docs.rs/sqlx/latest/sqlx/ |
| sqlx 源码与 issue | 社区与示例 | https://github.com/launchbadge/sqlx |
| Executor | 谁都能 `.execute()`：连接 / 事务 / 池 | https://docs.rs/sqlx/latest/sqlx/trait.Executor.html |
| Serde | 行 → struct 时常用（本阶段课内纯 std） | https://serde.rs/ |
| Rust 视频 | 补充听/看 | https://www.youtube.com/@RustVideos |

## 与真实 sqlx 的对应关系

| 本课概念 | 生产里大致是什么 |
| --- | --- |
| `UserRepository` + `InMemoryUserRepo` | `impl UserRepository for PgRepo { pool: Pool<Postgres> }` |
| `BoundQuery` + `SqlValue` | `sqlx::query(...).bind(x).bind(y)` |
| `SqlExecutor::fetch_optional` | `query.fetch_optional(&pool)` / `fetch_one` |
| `AccountStore::begin/commit/rollback` | `pool.begin().await?` → `tx.commit().await?` |

## 运行单课

```bash
cargo run --manifest-path 09_persistence/00_repo/Cargo.toml
cargo run --manifest-path 09_persistence/01_sqlx/Cargo.toml
cargo run --manifest-path 09_persistence/02_tx/Cargo.toml
```

**本目录总课数：3（已完成 3）**