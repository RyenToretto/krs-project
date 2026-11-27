# `21_architecture` | 架构与 SRE 收官

- **目标**: DDD 战术建模（实体/值对象/聚合根）、领域事件、ADR 评审清单、SRE（SLO/错误预算/发布清单）——纯 `std`，可离线 `cargo run` 自测。
- **前置**: 就业主线至 `15_project_capstone`；精通线建议先完成 `16_production_rust`、`14_observability`。
- **环境**: rustup 最新稳定版；验证：

```bash
./scripts/verify_modules.sh 21_architecture
```

- **推荐资料（明文 URL，已核验）**:
- https://rust-lang.github.io/api-guidelines/
- https://sre.google/books/
- https://nostarch.com/rust-rustaceans
- https://www.amazon.com/s?k=Rust+for+Rustaceans

## 本课 ↔ 生产实践

| 本仓库（内存模型 + `run_tests`） | 生产 / 团队协作 |
| --- | --- |
| `Order` 聚合根 + `Money` 值对象 | 限界上下文内的领域 crate，持久化在基础设施层 |
| `take_pending_events` | 事务提交 + Outbox / Kafka 投递 |
| `validate_adr_checklist` | `docs/adr/` PR 模板 + 架构评审会 |
| `SloPolicy` + `SreLaunchChecklist` | SLO 看板、错误预算政策、发布冻结与 on-call |

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_ddd/src/main.rs](./00_ddd/src/main.rs) | 实体 ID、值对象相等、聚合根不变量 | ✅ |
| `01_` | [01_events/src/main.rs](./01_events/src/main.rs) | 强类型领域事件、聚合缓冲、进程内总线 | ✅ |
| `02_` | [02_adr/src/main.rs](./02_adr/src/main.rs) | ADR 结构、Markdown 导出、可测评审清单 | ✅ |
| `03_` | [03_sre/src/main.rs](./03_sre/src/main.rs) | SLI/SLO、错误预算、燃尽速率、发布清单 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 明文 URL |
| --- | --- |
| Rust API Guidelines | https://rust-lang.github.io/api-guidelines/ |
| API Guidelines — Naming | https://rust-lang.github.io/api-guidelines/naming.html |
| API Guidelines — Interoperability | https://rust-lang.github.io/api-guidelines/interoperability.html |
| Google SRE Books | https://sre.google/books/ |
| SRE Workbook（站点） | https://sre.google/workbook/table-of-contents/ |
| Rust for Rustaceans（No Starch） | https://nostarch.com/rust-rustaceans |
| Amazon — Rust for Rustaceans 搜索 | https://www.amazon.com/s?k=Rust+for+Rustaceans |
| ADR 组织（社区习惯） | https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions |
| DDD 参考（概念） | https://martinfowler.com/bliki/DomainDrivenDesign.html |

**本目录总课数：4（已完成 4）**

## 学习顺序建议

1. `00_ddd`：先分清 **标识** vs **值**；所有修改走聚合根，拒绝非法状态。
2. `01_events`：事件是 **已发生事实**；在聚合内记录，提交后一次性取走再分发。
3. `02_adr`：重大技术选型先写 ADR，用清单拦住「只有一句话的决定」。
4. `03_sre`：上线前算清 **错误预算** 与 **燃尽速率**，清单不过不发布。

## 精通线收官提示

- 架构课不是「画框图」，而是 **可执行的约束**：类型、清单、SLO 数字都要能测。
- Rust 服务端典型分层：domain（本阶段）→ application → infrastructure（Axum/sqlx 在就业主线已练）。