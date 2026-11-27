/*
[主题]: Clippy 习惯 + API Guidelines 清单（可编程自检）
[难度]: Medium
[前置]: `05_modules`（可见性）、`08_web_axum`（public API 设计）

一、主题说明
- **Clippy** 是 Rust 内置 lint 集合：`cargo clippy` 在 CI 里常设 `-D warnings`。
- **API Guidelines** 约束**对外 crate API**：错误用 `Error` trait、构造函数命名、`must_use`、避免 `RefCell` 泄露等。
- 本课用**纯函数清单**模拟常见 `clippy::` 与 Guidelines 条目，对示例 struct 跑断言（不调用外部 clippy 二进制，verify 离线）。

二、写法对比
1. 直观版: 只靠肉眼 code review。
2. 进阶版: `ApiGuidelineReport` 聚合 `Violation`，bootstrap 打印人类可读报告。

三、真实用途与生产对照
| 本课 | 生产组件 |
| --- | --- |
| `check_new_without_default` | `clippy::new_without_default` |
| `check_must_use_on_result` | `clippy::must_use_candidate` |
| `C-COMMON-TRAITS` | 公有类型实现 `Debug` |
| `C-GOOD-ERR` | 公有错误实现 `std::error::Error` |
| CI | `cargo clippy -- -D clippy::all` |

资料: https://doc.rust-lang.org/clippy/
      https://rust-lang.github.io/rust-clippy/master/
      https://rust-lang.github.io/api-guidelines/
*/

use std::error::Error;
use std::fmt;

// ══════════════════════════════════════════════════════════════════
// 样本 API：一份「故意有问题」一份「合规」
// ══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct UserId(u64);

/// 反例风格 API
struct BadWidget {
    inner: u64,
}

impl BadWidget {
    fn new(x: u64) -> Self {
        Self { inner: x }
    }
    // 无 Default，触发 new_without_default 类检查
}

/// 良好 API 对照
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GoodWidget {
    inner: u64,
}

impl GoodWidget {
    pub fn new(inner: u64) -> Self {
        Self { inner }
    }
}

impl Default for GoodWidget {
    fn default() -> Self {
        Self { inner: 0 }
    }
}

#[derive(Debug)]
enum BadError {
    Oops,
}

impl fmt::Display for BadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "oops")
    }
}
// 故意不 impl Error —— 违反 C-GOOD-ERR

// 手写错误类型（无 thiserror 依赖，verify 纯离线）
#[derive(Debug, Clone, PartialEq, Eq)]
enum GoodErrorManual {
    Invalid(u64),
}

impl fmt::Display for GoodErrorManual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GoodErrorManual::Invalid(v) => write!(f, "无效 widget: {v}"),
        }
    }
}

impl Error for GoodErrorManual {}

#[must_use = "创建用户 id 失败时应处理 Result"]
fn parse_user_id(s: &str) -> Result<UserId, GoodErrorManual> {
    let n: u64 = s.parse().map_err(|_| GoodErrorManual::Invalid(0))?;
    if n == 0 {
        return Err(GoodErrorManual::Invalid(n));
    }
    Ok(UserId(n))
}

// ══════════════════════════════════════════════════════════════════
// 清单检查器（模拟 clippy + guidelines）
// ══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
enum GuidelineId {
    NewWithoutDefault,
    MissingErrorTrait,
    MissingDebug,
    MustUseOnResultCtor,
    RedundantClonePattern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    id: GuidelineId,
    item: &'static str,
    hint: &'static str,
}

struct ApiSurface {
    name: &'static str,
    has_default_impl: bool,
    has_public_new: bool,
    error_impls_std_error: bool,
    implements_debug: bool,
    result_fn_has_must_use: bool,
    allows_redundant_clone: bool,
}

impl ApiSurface {
    fn violations(&self) -> Vec<Violation> {
        let mut v = Vec::new();
        if self.has_public_new && !self.has_default_impl {
            v.push(Violation {
                id: GuidelineId::NewWithoutDefault,
                item: self.name,
                hint: "实现 Default 或改名为 with_* / builder",
            });
        }
        if self.name.contains("Error") && !self.error_impls_std_error {
            v.push(Violation {
                id: GuidelineId::MissingErrorTrait,
                item: self.name,
                hint: "公有错误应 impl std::error::Error（C-GOOD-ERR）",
            });
        }
        if !self.implements_debug {
            v.push(Violation {
                id: GuidelineId::MissingDebug,
                item: self.name,
                hint: "公有类型应 derive Debug（C-COMMON-TRAITS）",
            });
        }
        if !self.result_fn_has_must_use {
            v.push(Violation {
                id: GuidelineId::MustUseOnResultCtor,
                item: self.name,
                hint: "返回 Result 的构造函数建议 #[must_use]",
            });
        }
        if self.allows_redundant_clone {
            v.push(Violation {
                id: GuidelineId::RedundantClonePattern,
                item: self.name,
                hint: "优先借用 &.to_owned() 仅在必要时 clone（clippy::redundant_clone）",
            });
        }
        v
    }
}

struct ApiGuidelineReport {
    violations: Vec<Violation>,
}

impl ApiGuidelineReport {
    fn scan(surfaces: &[ApiSurface]) -> Self {
        let mut violations = Vec::new();
        for s in surfaces {
            violations.extend(s.violations());
        }
        Self { violations }
    }

    fn count(&self, id: GuidelineId) -> usize {
        self.violations.iter().filter(|v| v.id == id).count()
    }
}

/// 迷你静态分析：`.clone()` 后立即传引用（教学用字符串匹配）
fn lint_redundant_clone_snippet(code: &str) -> bool {
    code.contains(".clone()") && code.contains("foo(&")
}

fn clippy_style_from_bounds(x: Option<u32>) -> u32 {
    // 惯用：map_or / unwrap_or 代替冗长 match（对照 clippy::manual_map）
    x.map_or(0, |v| v)
}

fn guideline_public_fields_smell() -> &'static str {
    // 公有字段降低封装 —— 更倾向 getter / builder（见 guidelines Type Safety）
    "pub struct Config { pub db_url: String } // 谨慎"
}

fn run_tests() {
    let bad = [
        ApiSurface {
            name: "BadWidget",
            has_default_impl: false,
            has_public_new: true,
            error_impls_std_error: true,
            implements_debug: false,
            result_fn_has_must_use: true,
            allows_redundant_clone: false,
        },
        ApiSurface {
            name: "BadError",
            has_default_impl: false,
            has_public_new: false,
            error_impls_std_error: false,
            implements_debug: true,
            result_fn_has_must_use: true,
            allows_redundant_clone: false,
        },
    ];
    let report_bad = ApiGuidelineReport::scan(&bad);
    assert!(
        report_bad.count(GuidelineId::NewWithoutDefault) >= 1,
        "BadWidget 应触发 new_without_default"
    );
    assert!(
        report_bad.count(GuidelineId::MissingErrorTrait) >= 1,
        "BadError 缺 Error trait"
    );
    assert!(
        report_bad.count(GuidelineId::MissingDebug) >= 1,
        "BadWidget 无 Debug"
    );

    let good = [ApiSurface {
        name: "GoodWidget",
        has_default_impl: true,
        has_public_new: true,
        error_impls_std_error: true,
        implements_debug: true,
        result_fn_has_must_use: true,
        allows_redundant_clone: false,
    }];
    let report_good = ApiGuidelineReport::scan(&good);
    assert!(
        report_good.violations.is_empty(),
        "GoodWidget 应零违规 {:?}",
        report_good.violations
    );

    assert_eq!(clippy_style_from_bounds(Some(3)), 3);
    assert_eq!(clippy_style_from_bounds(None), 0);

    let smell = r#"
        let s = name.clone();
        foo(&s);
    "#;
    assert!(lint_redundant_clone_snippet(smell));

    assert_eq!(parse_user_id("42").unwrap(), UserId(42));
    assert!(parse_user_id("0").is_err());

    let _ = GoodWidget::default();
    let _ = GoodWidget::new(1);

    println!("✅ l00_clippy_api_guidelines 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ Clippy + API Guidelines 清单 ************\n");

    let surfaces = [
        ApiSurface {
            name: "BadWidget",
            has_default_impl: false,
            has_public_new: true,
            error_impls_std_error: true,
            implements_debug: false,
            result_fn_has_must_use: false,
            allows_redundant_clone: true,
        },
        ApiSurface {
            name: "GoodWidget",
            has_default_impl: true,
            has_public_new: true,
            error_impls_std_error: true,
            implements_debug: true,
            result_fn_has_must_use: true,
            allows_redundant_clone: false,
        },
    ];
    let report = ApiGuidelineReport::scan(&surfaces);
    for v in &report.violations {
        println!("[{:?}] {} — {}", v.id, v.item, v.hint);
    }

    println!("\n本地 CI 建议:");
    println!("  cargo clippy --all-targets -- -D warnings");
    println!("  rustdoc 链接检查 + API Guidelines 表格自查");
    println!("\n陷阱摘录: {}", guideline_public_fields_smell());

    run_tests();
}

fn main() {
    bootstrap();
}