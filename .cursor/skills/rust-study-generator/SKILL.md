---
name: rust-study-generator
description: >-
  Generate structured Rust learning modules for krs-project (00_rust_basics …
  21_architecture). Produces NN_ dirs, Cargo lesson packages with Chinese
  comments, run_tests()+bootstrap()+main, and scripts/verify_modules.sh. Use when
  initializing or expanding any NN_xxx stage, running full-auto roadmap generation,
  or auditing lessons.
---

# Rust Study Generator (`krs-project`)

为 `krs-project` 生成可直接运行的 Rust 学习课：中文注释、直观版 + 进阶版、`run_tests` + `bootstrap` + `main`、`./scripts/verify_modules.sh` 可过。

**终点目标（硬性）**：按顺序学完后，不止可就业 Rust 后端/系统向，还要沿精通线达到各方向**顶尖精通**（ownership/类型系统、async、Axum+sqlx、中间件、测试可观测、生产工程、Pin/Future、unsafe/FFI、性能、云原生、架构）。禁止浅尝辄止的「只贴框架皮毛」。

## 何时启用

- 「按 rust-study-generator / krs 学习规范初始化 `00_rust_basics` / `08_web_axum` …」
- 「全自动生成下一阶段 / 把路线图全部做完 / 推进 16→17→18→19→20→21」
- 在 `krs-project` 内新增编号学习目录或审计课是否合格

## Workflow

1. **读路线图，定目录名与课表**
   - 根 [README.md](../../../README.md)、[references/roadmap.md](references/roadmap.md)、[references/resources.md](resources.md)。
   - 阶段目录：`<repo-root>/NN_<slug>/`（命名可增补，顺序勿乱）。
   - 起点：**Rust 小白**；先就业主线 `00_`～`15_`，再精通线 **`16→17→18→19→20→21`**。
   - 工具链：`rustup` 安装最新稳定 `rustc`/`cargo`。
   - 每阶段课表以 roadmap「课表明细」为准；可增补，不可擅自删减核心知识点。

2. **读模板再写代码**
   - 必读 [references/templates.md](references/templates.md)。
   - 必读 [.cursor/rules/krs-learning-standards.mdc](../../rules/krs-learning-standards.mdc)。

3. **源码落盘约定**
   - 优先：`NN_slug/<topic>/Cargo.toml` + `src/main.rs`（包名 `l00_...`）。
   - 生产框架阶段：可用纯 Rust 迷你实现讲心智，README 写明与 Axum / sqlx / 真实中间件对照。

4. **生成内容约束**
   - 顶部主题块、中文注释、有价值时直观版 + 进阶版。
   - `run_tests()` 失败须 panic（或明确错误退出）；打印 `✅ ...通过`。
   - `bootstrap()` 演示后调用 `run_tests()`；`main` → `bootstrap()`。
   - 离线可测；新依赖写入该课 `Cargo.toml`。
   - 更新阶段 README 与根 README「已生成」表。
   - **外链**：明文完整 URL；须来自 resources.md 或同次 HTTP 核验。
   - 阶段 README 必须含：章节进度表 + 本阶段知识点资料包。

5. **分批提交（用户要求全自动时）**
   - 每完成一个 `NN_xxx`：verify 通过 → 单独 commit；全部完成再 push（用户明确要求时）。
   - 不要 force push；不要无关重构。

6. **验证**

```bash
./scripts/verify_modules.sh 00_rust_basics
./scripts/verify_modules.sh 00_rust_basics/00_hello
```

## 用户一句话触发示例

```text
按 rust-study-generator / krs 学习规范初始化 00_rust_basics
```

```text
全自动把 README 阶段路线里还没生成的目录都做完，分批次提交并 push
```

## Additional Resources

- [references/templates.md](references/templates.md)
- [references/roadmap.md](references/roadmap.md)
- [references/resources.md](references/resources.md)
- 仓库 `scripts/verify_modules.sh`
