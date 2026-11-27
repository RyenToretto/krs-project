/*
[主题]: 剖析习惯 —— 计时点、计数器清单与优化闭环
[难度]: Medium
[前置]: `14_observability`（指标心智）、本阶段 `00_bench`、`01_alloc`

一、主题说明
- **剖析** = 在关键路径上打点（时间 + 次数），先找热点再动刀；PGO / LTO 是编译器侧优化，仍需你证明热点在哪。
- 本课用纯 std 做「迷你剖析器」：阶段 `Instant` + 自定义计数器；生产里对齐 `tracing` span、`metrics` histogram。
- 清单思维：每次性能 PR 回答 —— 测了什么、基线多少、改后多少、有没有误伤 correctness。

二、写法对比
1. 直观版: 到处 `println!` + 手动算差。
2. 进阶版: `StageTimer` 统一 `mark`/`finish`；`HotCounters` 累加；`profiling_checklist()` 可单测。

三、真实用途
- 网关 handler：parse / auth / db / encode 四段计时；配合采样 trace。
- 发布前：Release + 粗测 +（可选）PGO 训练集见 rustc 文档。
资料: https://doc.rust-lang.org/rustc/profile-guided-optimization.html
      https://docs.rs/criterion/latest/criterion/
      https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590
*/

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// 计时点名称（穷举比散字符串更易 refactor）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Stage {
    Parse,
    Validate,
    Work,
    Encode,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Stage::Parse => "parse",
            Stage::Validate => "validate",
            Stage::Work => "work",
            Stage::Encode => "encode",
        }
    }
}

/// 进阶版：记录每阶段耗时（从上一 mark 到本次 mark）
struct StageTimer {
    started: Instant,
    last: Instant,
    spans: HashMap<Stage, Duration>,
}

impl StageTimer {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            started: now,
            last: now,
            spans: HashMap::new(),
        }
    }

    fn mark(&mut self, stage: Stage) {
        let now = Instant::now();
        let delta = now.saturating_duration_since(self.last);
        self.spans.insert(stage, delta);
        self.last = now;
    }

    fn total(&self) -> Duration {
        self.started.elapsed()
    }

    fn get(&self, stage: Stage) -> Duration {
        self.spans.get(&stage).copied().unwrap_or(Duration::ZERO)
    }
}

/// 热路径计数器（类似 metrics 里的 counter）
#[derive(Debug, Default, Clone)]
struct HotCounters {
    requests: u64,
    cache_hit: u64,
    cache_miss: u64,
    bytes_out: u64,
}

impl HotCounters {
    fn hit_rate(&self) -> f64 {
        let denom = self.cache_hit + self.cache_miss;
        if denom == 0 {
            return 0.0;
        }
        self.cache_hit as f64 / denom as f64
    }
}

/// 模拟请求管道（故意让 work 最重，便于剖析教学）
fn handle_request(payload: &str, counters: &mut HotCounters) -> String {
    counters.requests += 1;

    let mut timer = StageTimer::new();

    let tokens: Vec<&str> = payload.split(',').collect();
    timer.mark(Stage::Parse);

    let ok = tokens.iter().all(|t| t.len() <= 32);
    timer.mark(Stage::Validate);
    if !ok {
        counters.cache_miss += 1;
        return format!("ERR total={:?}", timer.total());
    }

    let mut acc = 0_u64;
    for t in &tokens {
        for b in t.as_bytes() {
            acc = acc.wrapping_add(*b as u64);
        }
    }
    timer.mark(Stage::Work);

    let out = format!("OK sum={} n={}", acc, tokens.len());
    counters.bytes_out += out.len() as u64;
    counters.cache_hit += 1;
    timer.mark(Stage::Encode);

    // 调试时可打印各段；bootstrap 里统一展示
    std::hint::black_box((timer.get(Stage::Work), out.clone()));
    out
}

/// PR 前自检清单（表驱动可测）
#[derive(Debug, Clone, Copy)]
struct CheckItem {
    key: &'static str,
    ok: bool,
}

fn profiling_checklist(has_baseline: bool, has_timer: bool, has_release_note: bool) -> Vec<CheckItem> {
    vec![
        CheckItem {
            key: "有测量基线（Instant 或 Criterion）",
            ok: has_baseline,
        },
        CheckItem {
            key: "关键阶段有计时点（非到处 println）",
            ok: has_timer,
        },
        CheckItem {
            key: "注明 Debug/Release 与 PGO 是否使用",
            ok: has_release_note,
        },
    ]
}

fn all_checks_pass(items: &[CheckItem]) -> bool {
    items.iter().all(|i| i.ok)
}

fn run_pipeline_n(n: u32) -> (HotCounters, Duration) {
    let mut counters = HotCounters::default();
    let payload = "alpha,beta,gamma,delta";
    let start = Instant::now();
    for _ in 0..n {
        std::hint::black_box(handle_request(payload, &mut counters));
    }
    (counters, start.elapsed())
}

fn run_tests() {
    let mut counters = HotCounters::default();
    let out = handle_request("a,b,c", &mut counters);
    assert!(out.starts_with("OK"));
    assert_eq!(counters.requests, 1);
    assert_eq!(counters.cache_hit, 1);

    let bad = handle_request("this_field_name_is_way_too_long_for_policy,ok", &mut counters);
    assert!(bad.starts_with("ERR"));
    assert!(counters.cache_miss >= 1);

    let items = profiling_checklist(true, true, true);
    assert!(all_checks_pass(&items), "清单项应全过");

    let fail = profiling_checklist(false, true, true);
    assert!(!all_checks_pass(&fail));

    let (c, wall) = run_pipeline_n(3_000);
    assert_eq!(c.requests, 3_000);
    assert!(wall.as_secs() < 3, "管道粗测应在 3s 内 wall={:?}", wall);
    assert!(c.bytes_out > 0);

    println!("✅ 剖析习惯（计时点 / 计数器 / 清单）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L02 剖析习惯 ************\n");

    let mut counters = HotCounters::default();
    let sample = "id-1,id-2,id-3";
    let _ = handle_request(sample, &mut counters);

    let mut demo_timer = StageTimer::new();
    let _ = sample.split(',').collect::<Vec<_>>();
    demo_timer.mark(Stage::Parse);
    demo_timer.mark(Stage::Validate);
    demo_timer.mark(Stage::Work);
    demo_timer.mark(Stage::Encode);

    println!("单请求阶段耗时（演示用）:");
    for stage in [Stage::Parse, Stage::Validate, Stage::Work, Stage::Encode] {
        println!("  {} ≈ {:?}", stage.label(), demo_timer.get(stage));
    }

    let (c, wall) = run_pipeline_n(5_000);
    println!(
        "\n批处理 5000 次: wall={:?} hit_rate={:.2}% bytes_out={}",
        wall,
        c.hit_rate() * 100.0,
        c.bytes_out
    );

    println!("\n剖析 / 优化闭环清单:");
    for item in profiling_checklist(true, true, true) {
        println!("  [{}] {}", if item.ok { "x" } else { " " }, item.key);
    }

    println!("\n编译器侧 PGO（训练集 + 再编译）:");
    println!("  https://doc.rust-lang.org/rustc/profile-guided-optimization.html");
    println!("微基准库:");
    println!("  https://docs.rs/criterion/latest/criterion/\n");

    run_tests();
}

fn main() {
    bootstrap();
}