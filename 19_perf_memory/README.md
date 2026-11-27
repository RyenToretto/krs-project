# `19_perf_memory` | 性能与内存

- **目标**: 建立「先测量再优化」的闭环——`Instant` 教学粗测与 Criterion 分工、分配器直觉（`with_capacity` / 少 `clone`）、以及在关键路径上打计时点与计数器的剖析习惯（并对照 PGO 文档知道编译器还能做什么）。
- **前置**: 就业主线至 `15_project_capstone`；精通线建议已完成 `16_production_rust`、`17_async_advanced`、`18_unsafe_ffi`（本课不依赖 unsafe，但与生产调优衔接）。
- **环境**: rustup 最新稳定版；验证：

```bash
./scripts/verify_modules.sh 19_perf_memory
```

- **推荐资料（明文 URL，已核验）**:
- https://docs.rs/criterion/latest/criterion/
- https://doc.rust-lang.org/rustc/profile-guided-optimization.html
- https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590

## 本课 ↔ 生产实践

| 本仓库（`cargo run` + `run_tests`） | 生产 / 团队协作 |
| --- | --- |
| `Instant` + 中位采样 | `criterion` / `iai` benches + CI 对比 |
| `Vec::with_capacity` 对照 | 按 body/row count 预分配 buffer |
| `StageTimer` / `HotCounters` | `tracing` span + `metrics` histogram |
| `profiling_checklist()` | 性能 PR 模板 + 火焰图附件 |

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_bench/src/main.rs](./00_bench/src/main.rs) | `Instant` 粗测、预热/中位数、Criterion 迁移提示 | ✅ |
| `01_` | [01_alloc/src/main.rs](./01_alloc/src/main.rs) | `push` 扩容 vs 预分配、无谓 `to_vec`/`clone` | ✅ |
| `02_` | [02_profile/src/main.rs](./02_profile/src/main.rs) | 阶段计时点、热路径计数器、剖析清单 + PGO 链 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 明文 URL |
| --- | --- |
| Criterion 微基准 | https://docs.rs/criterion/latest/criterion/ |
| Rustc PGO | https://doc.rust-lang.org/rustc/profile-guided-optimization.html |
| Programming Rust（O'Reilly） | https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590 |
| `Vec::with_capacity` | https://doc.rust-lang.org/std/vec/struct.Vec.html#method.with_capacity |
| Book — 测试 | https://doc.rust-lang.org/book/ch11-00-testing.html |

**本目录总课数：3（已完成 3）**

## 学习顺序建议

1. `00_bench`：牢记 **Debug ≠ Release**；粗测只锁流程，阈值进 Criterion。
2. `01_alloc`：改性能前先看 **分配次数** 与是否可复制热路径上的 `String`。
3. `02_profile`：用清单保证每次优化 PR 有基线；需要编译器助力时再读 PGO 页。

## 测量边界（本仓库硬性）

- 禁止在 verify 路径跑分钟级 `cargo bench`；本阶段计时均在毫秒～数秒。
- 不把单次粗测当 CI 门槛；教学断言只防环境异常（超时过大即失败）。