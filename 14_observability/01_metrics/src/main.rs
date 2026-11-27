/*
[主题]: 指标心智 —— Counter / Histogram 与可导出注册表（纯 Rust 迷你实现）
[难度]: Medium
[前置]: `00_rust_basics`（HashMap）；`08_web_axum`（理解 QPS、延迟分位）

一、主题说明
- **日志（tracing）**：回答「发生了什么、在哪条调用链」—— 适合排障叙事。
- **指标（metrics）**：回答「有多少、多快、错误率」—— 适合告警与容量规划（Counter 只增、Histogram 看分位）。
- Prometheus 拉取格式：`name{label="v"} value`；直方图用 `_bucket` / `_sum` / `_count`。
- Rust 生态常见 `metrics` crate + `metrics-exporter-prometheus`；本课手写 `Registry` 秒级离线演示。

二、写法对比
1. 直观版: 全局 `static mut` 或一个 `u64` 自增 —— 无标签、无法多路由并存。
2. 进阶版: `MetricKey { name, labels }` + `Registry` 存 Counter/Histogram，`observe` 落入预定义桶。

三、真实用途
- `http_requests_total{method="GET",status="200"}` 做 QPS 与错误率。
- `http_request_duration_seconds` 直方图算 p50/p99，对照 SLO。
- 与 tracing 联动：同一 `route` 标签出现在日志字段与指标 label 上。
资料: https://docs.rs/tracing/latest/tracing/
      https://github.com/tokio-rs/tracing
      https://opentelemetry.io/docs/languages/rust/
*/

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

// ── 指标键：name + 有序 labels（对照 metrics::Key）────────────────────

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MetricKey {
    name: &'static str,
    labels: Vec<(&'static str, String)>,
}

impl MetricKey {
    fn new(name: &'static str, labels: Vec<(&'static str, String)>) -> Self {
        let mut labels = labels;
        labels.sort_by(|a, b| a.0.cmp(b.0));
        Self { name, labels }
    }
}

// ── Counter：单调递增 ────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
struct Counter {
    value: u64,
}

impl Counter {
    fn inc(&mut self, delta: u64) {
        self.value = self.value.saturating_add(delta);
    }

    fn get(&self) -> u64 {
        self.value
    }
}

// ── Histogram：固定桶 + sum/count（Prometheus 风格）────────────────────

#[derive(Clone, Debug)]
struct Histogram {
    /// 上界秒数（最后一桶常为 +Inf，本课用 f64::INFINITY 表示）
    bounds: Vec<f64>,
    counts: Vec<u64>,
    sum: f64,
    count: u64,
}

impl Histogram {
    fn new(bounds: Vec<f64>) -> Self {
        let mut b = bounds;
        b.sort_by(|x, y| x.partial_cmp(y).unwrap());
        let counts = vec![0; b.len()];
        Self {
            bounds: b,
            counts,
            sum: 0.0,
            count: 0,
        }
    }

    fn observe(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
        // Prometheus `_bucket` 为累积计数：每个 le 表示「≤ 该上界」的观测数
        for (i, bound) in self.bounds.iter().enumerate() {
            if value <= *bound {
                self.counts[i] += 1;
            }
        }
    }

    /// 粗略分位：落在桶内的累计比例（非 exact quantile）
    fn approximate_quantile(&self, q: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let target = (self.count as f64 * q).ceil() as u64;
        let mut acc = 0u64;
        for (i, &c) in self.counts.iter().enumerate() {
            acc += c;
            if acc >= target {
                return self.bounds[i];
            }
        }
        *self.bounds.last().unwrap_or(&0.0)
    }
}

enum MetricValue {
    Counter(Counter),
    Histogram(Histogram),
}

// ── 直观版：无 label 的总计数 ───────────────────────────────────────

struct NaiveTotal {
    hits: u64,
}

impl NaiveTotal {
    fn bump(&mut self) {
        self.hits += 1;
    }
}

// ── Registry：注册 + 写入 ───────────────────────────────────────────

struct Registry {
    metrics: HashMap<MetricKey, MetricValue>,
}

impl Registry {
    fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    fn counter(&mut self, name: &'static str, labels: Vec<(&'static str, String)>) -> &mut Counter {
        let key = MetricKey::new(name, labels);
        match self.metrics.entry(key).or_insert_with(|| MetricValue::Counter(Counter::default())) {
            MetricValue::Counter(c) => c,
            MetricValue::Histogram(_) => panic!("同名键类型冲突"),
        }
    }

    fn histogram(
        &mut self,
        name: &'static str,
        labels: Vec<(&'static str, String)>,
        bounds: Vec<f64>,
    ) -> &mut Histogram {
        let key = MetricKey::new(name, labels);
        match self.metrics.entry(key).or_insert_with(|| {
            MetricValue::Histogram(Histogram::new(bounds.clone()))
        }) {
            MetricValue::Histogram(h) => h,
            MetricValue::Counter(_) => panic!("同名键类型冲突"),
        }
    }

    fn render_prometheus_like(&self) -> String {
        let mut out = String::new();
        for (key, val) in &self.metrics {
            let label_str = if key.labels.is_empty() {
                String::new()
            } else {
                let inner: Vec<String> = key
                    .labels
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                format!("{{{}}}", inner.join(","))
            };
            match val {
                MetricValue::Counter(c) => {
                    let _ = writeln!(&mut out, "{}{} {}", key.name, label_str, c.get());
                }
                MetricValue::Histogram(h) => {
                    for (i, bound) in h.bounds.iter().enumerate() {
                        let le = if bound.is_finite() {
                            bound.to_string()
                        } else {
                            "+Inf".to_string()
                        };
                        let bucket_name = format!("{}_bucket", key.name);
                        let extra = if label_str.is_empty() {
                            format!("{{le=\"{}\"}}", le)
                        } else {
                            let inner = label_str.trim_start_matches('{').trim_end_matches('}');
                            format!("{{{},le=\"{}\"}}", inner, le)
                        };
                        let _ = writeln!(&mut out, "{}{} {}", bucket_name, extra, h.counts[i]);
                    }
                    let _ = writeln!(&mut out, "{}_sum{} {}", key.name, label_str, h.sum);
                    let _ = writeln!(&mut out, "{}_count{} {}", key.name, label_str, h.count);
                }
            }
        }
        out
    }
}

// ── 模拟 HTTP 层记录指标 ─────────────────────────────────────────────

fn record_http(reg: &mut Registry, method: &str, status: u16, duration_secs: f64) {
    let labels = vec![
        ("method", method.to_string()),
        ("status", status.to_string()),
    ];
  reg.counter("http_requests_total", labels.clone()).inc(1);
    let bounds = vec![0.005, 0.01, 0.025, 0.05, 0.1, f64::INFINITY];
    reg.histogram("http_request_duration_seconds", labels, bounds)
        .observe(duration_secs);
}

fn run_tests() {
    let mut naive = NaiveTotal { hits: 0 };
    naive.bump();
    naive.bump();
    assert_eq!(naive.hits, 2);

    let mut reg = Registry::new();
    record_http(&mut reg, "GET", 200, 0.008);
    record_http(&mut reg, "GET", 200, 0.042);
    record_http(&mut reg, "POST", 500, 0.12);

    let key_get = MetricKey::new(
        "http_requests_total",
        vec![("method", "GET".into()), ("status", "200".into())],
    );
    let cv = reg.metrics.get(&key_get).expect("GET 200 counter");
    let MetricValue::Counter(c) = cv else { panic!() };
    assert_eq!(c.get(), 2);

    let key_post = MetricKey::new(
        "http_requests_total",
        vec![("method", "POST".into()), ("status", "500".into())],
    );
    let MetricValue::Counter(c2) = reg.metrics.get(&key_post).unwrap() else { panic!() };
    assert_eq!(c2.get(), 1);

    let hist_key = MetricKey::new(
        "http_request_duration_seconds",
        vec![("method", "GET".into()), ("status", "200".into())],
    );
    let MetricValue::Histogram(h) = reg.metrics.get(&hist_key).unwrap() else { panic!() };
    assert_eq!(h.count, 2);
    assert!(h.approximate_quantile(0.5) <= 0.05);

    let rendered = reg.render_prometheus_like();
    assert!(rendered.contains("http_requests_total"));
    assert!(rendered.contains("http_request_duration_seconds_bucket"));

    println!("✅ 指标心智（Counter + Histogram + Registry 导出）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L01 metrics 心智 ************\n");

    println!("── 直观版：全局计数 ──");
    let mut naive = NaiveTotal { hits: 0 };
    for _ in 0..3 {
        naive.bump();
    }
    println!("naive hits = {}", naive.hits);

    println!("\n── 进阶版：带 label 的 Counter + Histogram ──");
    let mut reg = Registry::new();
    record_http(&mut reg, "GET", 200, 0.003);
    record_http(&mut reg, "GET", 200, 0.09);
    record_http(&mut reg, "DELETE", 404, 0.001);
    println!("Prometheus 风格文本（节选）:\n{}", reg.render_prometheus_like());

    println!("对照生产: metrics::counter!(\"http_requests_total\", \"method\" => \"GET\").increment(1);");
    println!("OTel Metrics API: https://opentelemetry.io/docs/languages/rust/\n");

    run_tests();
}

fn main() {
    bootstrap();
}