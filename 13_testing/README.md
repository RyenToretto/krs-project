# `13_testing` | 测试与基准心智

- **目标**: **表驱动单元测**、**trait Mock**、**集成测试心智**（同进程多模块联调）；粗计时入门，正式微基准指向 Criterion（本阶段无 criterion 依赖）。
- **前置**: `00_rust_basics`（assert）、`04_traits_generics`（Mock）、`08_web_axum` / `09_persistence`（分层与 Repo）——无则按课内注释自学。
- **环境**: rustup 最新稳定版；验证：

```bash
./scripts/verify_modules.sh 13_testing
```

- **推荐资料（明文 URL，已核验）**:
- https://doc.rust-lang.org/book/ch11-00-testing.html
- https://doc.rust-lang.org/rust-by-example/testing.html
- https://docs.rs/criterion/latest/criterion/

## 本课 ↔ 标准 Rust 测试布局

| 本仓库（`cargo run` + `run_tests`） | 生产 / 团队协作 |
| --- | --- |
| `main.rs` 内 `run_tests()` | `#[cfg(test)]` + `cargo test` |
| 表驱动数组 `const CASES` | 同左，或参数化测试 crate |
| `RecordingInventory` 手写 Mock | `mockall`、依赖注入容器 |
| `AppHarness` 同进程联调 | `tests/*.rs` 集成测试 crate + 可选 testcontainers |
| `Instant::now()` 粗基准 | `criterion` benches |

集成测试官方说明（明文 URL）: https://doc.rust-lang.org/book/ch11-02-running-tests.html

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_unit/src/main.rs](./00_unit/src/main.rs) | 表驱动数组、`SHARED`/`ADVANCED` 分表、粗计时 | ✅ |
| `01_` | [01_mock/src/main.rs](./01_mock/src/main.rs) | `Inventory` trait、录制调用、表驱动 Mock 用例 | ✅ |
| `02_` | [02_integration/src/main.rs](./02_integration/src/main.rs) | repo/service/api、`AppHarness`、表驱动端到端 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 明文 URL |
| --- | --- |
| Book 第 11 章 测试 | https://doc.rust-lang.org/book/ch11-00-testing.html |
| 运行测试 / 集成测目录 | https://doc.rust-lang.org/book/ch11-02-running-tests.html |
| Rust By Example 测试 | https://doc.rust-lang.org/rust-by-example/testing.html |
| Criterion 微基准（生产） | https://docs.rs/criterion/latest/criterion/ |

**本目录总课数：3（已完成 3）**

## 学习顺序建议

1. `00_unit`：先会写 **Case 结构体 + const 数组**，再考虑是否拆 `SHARED` / 仅进阶用例表。
2. `01_mock`：业务只依赖 **trait**；断言「返回值」+「调用次数」。
3. `02_integration`：对照如何把模块搬进 `lib.rs` + `tests/`，再给 `12_grpc_tonic` / `15_project_capstone` 写联调。

## 下一步（可选）

在 capstone 或工作项目里：`cargo add criterion --dev`，把 `00_unit` 里的 `parse_cents` 写成 `benches/parse.rs`；集成测目录里用 `AppHarness` 同款布线对接 axum `Router` + `TestClient`（需 `axum` dev 依赖，非本阶段 verify 要求）。