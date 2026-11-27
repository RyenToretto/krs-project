/*
[主题]: 架构决策记录（ADR）—— 结构化模板 + 可测检查清单
[难度]: Medium
[前置]: `05_cargo_modules`、`16_production_rust`；团队协作心智

一、主题说明
- **ADR** 记录「在什么上下文下做了什么架构决定、后果是什么」，避免口头传说与重复争论。
- 常用字段：标题、状态（Proposed/Accepted/Deprecated）、背景、决策、后果、备选方案。
- **检查清单** 用于 PR / 架构评审：缺字段、空决策、无后果说明 → 拒绝合并。
- 本课用纯 `struct` + 校验函数模拟团队仓库里的 `docs/adr/0001-*.md` 前置校验。

二、写法对比
1. 直观版: 用键值对 `HashMap` 存 ADR 字段，快速拼接 Markdown 字符串。
2. 进阶版: `ArchitectureDecisionRecord` 强类型 + `validate_adr_checklist()` 返回逐项 `CheckItem`。

三、真实用途
- 选 Axum vs Actix、sqlx vs diesel、单体 vs 拆分服务前写 ADR；与 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) 的「清晰契约」一致。
资料: https://rust-lang.github.io/api-guidelines/
      https://sre.google/books/
      https://nostarch.com/rust-rustaceans
      https://www.amazon.com/s?k=Rust+for+Rustaceans
*/

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// ADR 状态
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdrStatus {
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
}

impl AdrStatus {
    fn as_str(self) -> &'static str {
        match self {
            AdrStatus::Proposed => "Proposed",
            AdrStatus::Accepted => "Accepted",
            AdrStatus::Deprecated => "Deprecated",
            AdrStatus::Superseded => "Superseded",
        }
    }
}

#[derive(Debug, Clone)]
struct ArchitectureDecisionRecord {
    number: u32,
    title: String,
    status: AdrStatus,
    context: String,
    decision: String,
    consequences: String,
    alternatives: String,
}

impl ArchitectureDecisionRecord {
    fn markdown(&self) -> String {
        format!(
            "# ADR {num:04}: {title}\n\n\
             **Status**: {status}\n\n\
             ## Context\n{context}\n\n\
             ## Decision\n{decision}\n\n\
             ## Consequences\n{consequences}\n\n\
             ## Alternatives\n{alternatives}\n",
            num = self.number,
            title = self.title,
            status = self.status.as_str(),
            context = self.context,
            decision = self.decision,
            consequences = self.consequences,
            alternatives = self.alternatives,
        )
    }
}

/// 直观版：松散字段
#[derive(Debug, Default)]
struct NaiveAdrMap {
    fields: HashMap<&'static str, String>,
}

impl NaiveAdrMap {
    fn to_markdown(&self) -> String {
        let title = self.fields.get("title").map(String::as_str).unwrap_or("(无标题)");
        let decision = self.fields.get("decision").map(String::as_str).unwrap_or("");
        format!("# {title}\n\n## Decision\n{decision}\n")
    }
}

// ---------------------------------------------------------------------------
// 检查清单
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct CheckItem {
    name: &'static str,
    ok: bool,
}

#[derive(Debug)]
struct ChecklistReport {
    items: Vec<CheckItem>,
}

impl ChecklistReport {
    fn all_passed(&self) -> bool {
        self.items.iter().all(|c| c.ok)
    }

    fn failed_names(&self) -> Vec<&'static str> {
        self.items.iter().filter(|c| !c.ok).map(|c| c.name).collect()
    }
}

const MIN_SECTION_LEN: usize = 20;

fn validate_adr_checklist(adr: &ArchitectureDecisionRecord) -> ChecklistReport {
    let items = vec![
        CheckItem {
            name: "编号为正",
            ok: adr.number > 0,
        },
        CheckItem {
            name: "标题非空",
            ok: !adr.title.trim().is_empty(),
        },
        CheckItem {
            name: "状态已设置",
            ok: matches!(
                adr.status,
                AdrStatus::Proposed
                    | AdrStatus::Accepted
                    | AdrStatus::Deprecated
                    | AdrStatus::Superseded
            ),
        },
        CheckItem {
            name: "背景足够长",
            ok: adr.context.trim().len() >= MIN_SECTION_LEN,
        },
        CheckItem {
            name: "决策足够长",
            ok: adr.decision.trim().len() >= MIN_SECTION_LEN,
        },
        CheckItem {
            name: "后果已写",
            ok: adr.consequences.trim().len() >= MIN_SECTION_LEN,
        },
        CheckItem {
            name: "备选方案已写",
            ok: adr.alternatives.trim().len() >= MIN_SECTION_LEN,
        },
        CheckItem {
            name: "Markdown 含 ADR 头",
            ok: adr.markdown().contains("# ADR"),
        },
    ];
    ChecklistReport { items }
}

fn sample_accepted_adr() -> ArchitectureDecisionRecord {
    ArchitectureDecisionRecord {
        number: 12,
        title: "采用 Axum 作为 HTTP 框架".to_string(),
        status: AdrStatus::Accepted,
        context: "团队需要异步优先、生态与 Tower 中间件对齐的 Web 层；已有成员有 Tokio 经验。"
            .to_string(),
        decision: "选用 Axum 0.7+ 作为新服务默认 HTTP 栈；旧 Actix 服务保持维护模式不迁。"
            .to_string(),
        consequences: "正面：与 tracing/tower 统一；负面：需统一错误类型与 extractor 约定培训。"
            .to_string(),
        alternatives: "Actix-web：吞吐成熟但中间件模型与 Tower 分叉；Rocket：易上手但 async 栈差异。"
            .to_string(),
    }
}

fn run_tests() {
    let good = sample_accepted_adr();
    let report = validate_adr_checklist(&good);
    assert!(report.all_passed(), "合法 ADR 应全过: {:?}", report.failed_names());

    let bad = ArchitectureDecisionRecord {
        number: 0,
        title: String::new(),
        status: AdrStatus::Proposed,
        context: "短".to_string(),
        decision: "未定".to_string(),
        consequences: String::new(),
        alternatives: String::new(),
    };
    let bad_report = validate_adr_checklist(&bad);
    assert!(!bad_report.all_passed());
    assert!(bad_report.failed_names().len() >= 5);

    let mut naive = NaiveAdrMap::default();
    naive
        .fields
        .insert("title", "ADR 0001 Plain Map".into());
    naive
        .fields
        .insert("decision", "继续用纯 Rust 标准库写领域层".into());
    let md = naive.to_markdown();
    assert!(md.contains("Plain Map"));

    let md_full = good.markdown();
    assert!(md_full.contains("ADR 0012"));
    assert!(md_full.contains("Accepted"));

    println!("✅ adr_checklist 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ ADR 模板与评审清单 ************\n");

    let adr = sample_accepted_adr();
    let report = validate_adr_checklist(&adr);
    println!(
        "ADR #{} 清单: {} / {} 项通过",
        adr.number,
        report.items.iter().filter(|c| c.ok).count(),
        report.items.len()
    );
    for item in &report.items {
        let mark = if item.ok { "✓" } else { "✗" };
        println!("  [{mark}] {}", item.name);
    }

    println!("\n--- Markdown 预览（截取）---");
    let preview: String = adr.markdown().chars().take(280).collect();
    println!("{preview}...");

    run_tests();
}

fn main() {
    bootstrap();
}