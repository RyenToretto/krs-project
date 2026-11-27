/*
[主题]: SRE 检查清单 —— SLI/SLO、错误预算、燃尽速率（可算可测）
[难度]: Medium
[前置]: `14_observability`（指标心智）；`16_production_rust`（韧性）

一、主题说明
- **SLI**（指标）：用户感知的服务质量度量，如成功率、延迟 P99。
- **SLO**（目标）：SLI 在时间窗口内的目标值，如「30 天内 99.9% 请求成功」。
- **错误预算（Error Budget）**：允许的不达标额度 = `1 - SLO`；预算耗尽应 **放缓发布、投入可靠性**。
- **燃尽（Burn rate）**：窗口内错误速率相对预算的倍数；多窗口告警（Google SRE Book）可映射为纯算术自检。
- 本课不算真实 Prometheus 数据，而用 **可重复输入 → 断言** 固化心智。

二、写法对比
1. 直观版: `f64` 手算成功率与剩余预算比例。
2. 进阶版: `SloPolicy` + `ErrorBudgetLedger` + `SreLaunchChecklist`（发布前勾选项）。

三、真实用途
- 与 on-call 手册、发布冻结策略联动；Rust 服务同样要为 p99、停机窗口定 SLO。
资料: https://sre.google/books/
      https://rust-lang.github.io/api-guidelines/
      https://nostarch.com/rust-rustaceans
      https://www.amazon.com/s?k=Rust+for+Rustaceans
*/

// ---------------------------------------------------------------------------
// SLI / SLO 与错误预算
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct SloPolicy {
    /// 目标可用性，如 0.999 表示 99.9%
    target_availability: f64,
    /// 滚动窗口天数（教学用 30）
    window_days: u32,
}

impl SloPolicy {
    fn error_budget_fraction(self) -> f64 {
        1.0 - self.target_availability
    }

    /// 按「总请求数」折算允许失败次数（向下取整）
    fn allowed_failures(self, total_requests: u64) -> u64 {
        let budget = self.error_budget_fraction();
        (total_requests as f64 * budget).floor() as u64
    }
}

#[derive(Debug, Clone, Copy)]
struct SliWindow {
    total_requests: u64,
    failed_requests: u64,
    /// 观测窗口长度（小时），用于燃尽速率
    window_hours: u32,
}

impl SliWindow {
    fn success_rate(self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        let success = self
            .total_requests
            .saturating_sub(self.failed_requests);
        success as f64 / self.total_requests as f64
    }

    fn meets_slo(self, slo: SloPolicy) -> bool {
        self.success_rate() >= slo.target_availability
    }

    /// 燃尽速率：实际失败占比 / 错误预算占比（>1 表示烧预算快于匀速）
    fn burn_rate(self, slo: SloPolicy) -> f64 {
        let budget = slo.error_budget_fraction();
        if budget <= 0.0 || self.total_requests == 0 {
            return 0.0;
        }
        let actual_error_frac = self.failed_requests as f64 / self.total_requests as f64;
        actual_error_frac / budget
    }

    fn remaining_budget_requests(self, slo: SloPolicy) -> i64 {
        let allowed = slo.allowed_failures(self.total_requests) as i64;
        allowed - self.failed_requests as i64
    }
}

/// 直观版：仅算成功率与预算余量
fn naive_budget_left(successes: u64, failures: u64, target: f64) -> f64 {
    let total = successes + failures;
    if total == 0 {
        return 1.0;
    }
    let rate = successes as f64 / total as f64;
    let budget = 1.0 - target;
    let used = (1.0 - rate).max(0.0);
    (budget - used).max(0.0) / budget
}

#[derive(Debug, Clone, Copy)]
struct ErrorBudgetLedger {
    total_requests: u64,
    consumed_failures: u64,
}

impl ErrorBudgetLedger {
    fn consume(mut self, slo: SloPolicy, new_failures: u64) -> Result<Self, &'static str> {
        let next = self
            .consumed_failures
            .checked_add(new_failures)
            .ok_or("失败计数溢出")?;
        let cap = slo.allowed_failures(self.total_requests);
        if next > cap {
            return Err("错误预算已耗尽");
        }
        self.consumed_failures = next;
        Ok(self)
    }

    fn remaining(self, slo: SloPolicy) -> u64 {
        slo.allowed_failures(self.total_requests)
            .saturating_sub(self.consumed_failures)
    }
}

// ---------------------------------------------------------------------------
// 发布前 SRE 清单
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct CheckItem {
    name: &'static str,
    ok: bool,
}

#[derive(Debug)]
struct SreLaunchChecklist {
    items: Vec<CheckItem>,
}

impl SreLaunchChecklist {
    fn ready_for_release(&self) -> bool {
        self.items.iter().all(|c| c.ok)
    }
}

fn build_launch_checklist(
    slo: SloPolicy,
    window: SliWindow,
    has_runbook: bool,
    has_rollback: bool,
    on_call_named: bool,
) -> SreLaunchChecklist {
    let budget_ok = window.remaining_budget_requests(slo) > 0;
    let burn_ok = window.burn_rate(slo) < 14.0;
    let slo_ok = window.meets_slo(slo);

    SreLaunchChecklist {
        items: vec![
            CheckItem {
                name: "当前窗口满足 SLO",
                ok: slo_ok,
            },
            CheckItem {
                name: "错误预算未耗尽",
                ok: budget_ok,
            },
            CheckItem {
                name: "燃尽速率未超教学阈值",
                ok: burn_ok,
            },
            CheckItem {
                name: "有 Runbook",
                ok: has_runbook,
            },
            CheckItem {
                name: "有回滚方案",
                ok: has_rollback,
            },
            CheckItem {
                name: "值班人已指定",
                ok: on_call_named,
            },
        ],
    }
}

const THREE_NINES: SloPolicy = SloPolicy {
    target_availability: 0.999,
    window_days: 30,
};

fn run_tests() {
    assert!((THREE_NINES.error_budget_fraction() - 0.001).abs() < 1e-9);
    assert_eq!(THREE_NINES.allowed_failures(1_000_000), 1_000);

    let healthy = SliWindow {
        total_requests: 100_000,
        failed_requests: 50,
        window_hours: 24,
    };
    assert!(healthy.meets_slo(THREE_NINES));
    assert!(healthy.burn_rate(THREE_NINES) < 1.0);
    assert!(healthy.remaining_budget_requests(THREE_NINES) > 0);

    let hot = SliWindow {
        total_requests: 10_000,
        failed_requests: 200,
        window_hours: 1,
    };
    assert!(!hot.meets_slo(THREE_NINES));
    assert!(hot.burn_rate(THREE_NINES) > 10.0);

    let exhausted = naive_budget_left(9990, 10, 0.999);
    assert!((exhausted - 0.0).abs() < 1e-9);
    let left = naive_budget_left(9999, 1, 0.999);
    assert!(left > 0.0 && left <= 1.0);

    let ledger = ErrorBudgetLedger {
        total_requests: 1_000_000,
        consumed_failures: 900,
    };
    assert_eq!(ledger.remaining(THREE_NINES), 100);
    assert!(ledger.consume(THREE_NINES, 50).is_ok());
    assert!(ledger.consume(THREE_NINES, 200).is_err());

    let checklist = build_launch_checklist(THREE_NINES, healthy, true, true, true);
    assert!(checklist.ready_for_release());

    let bad_launch = build_launch_checklist(THREE_NINES, hot, true, false, true);
    assert!(!bad_launch.ready_for_release());

    let zero = SliWindow {
        total_requests: 0,
        failed_requests: 0,
        window_hours: 1,
    };
    assert_eq!(zero.success_rate(), 1.0);

    println!("✅ sre_checklist 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ SRE：SLO、错误预算、发布清单 ************\n");

    let window = SliWindow {
        total_requests: 500_000,
        failed_requests: 300,
        window_hours: 24,
    };
    println!(
        "成功率: {:.4}%",
        window.success_rate() * 100.0
    );
    println!(
        "错误预算允许失败: {} 次",
        THREE_NINES.allowed_failures(window.total_requests)
    );
    println!(
        "剩余预算（次）: {}",
        window.remaining_budget_requests(THREE_NINES)
    );
    println!("燃尽速率: {:.2}x", window.burn_rate(THREE_NINES));

    let checklist = build_launch_checklist(THREE_NINES, window, true, true, true);
    println!(
        "发布就绪: {}",
        if checklist.ready_for_release() {
            "是"
        } else {
            "否"
        }
    );

    run_tests();
}

fn main() {
    bootstrap();
}