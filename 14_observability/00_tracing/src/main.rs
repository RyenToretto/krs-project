/*
[主题]: tracing 心智 —— 结构化日志事件与 Span 上下文栈（纯 Rust 迷你实现）
[难度]: Medium
[前置]: `07_async_tokio`（理解异步里「请求链路」）；`05_cargo_modules`（模块与 trait）

一、主题说明
- **println 日志**：只能打字符串，不知道「属于哪次请求」、难以按级别过滤、无法关联父/子调用。
- **tracing**：把日志升级成 **Event（事件）** 与 **Span（区间）**；Event 挂在当前 Span 上，形成可检索的调用树。
- 核心概念：`Level`（过滤粒度）、`target`（模块/ crate 名）、`fields`（键值字段）、`Subscriber`（谁接收并渲染）。
- 真库用宏 `tracing::info!`、`#[instrument]`、`tracing_subscriber::Registry` + `Layer` 叠加输出。
- 本课不引 `tracing` crate，自己实现最小 Subscriber + Span 栈，秒级离线跑通。

二、写法对比
1. 直观版: `println!("[INFO] msg")` —— 无上下文、无字段结构。
2. 进阶版: `MiniSpan` 入栈 + `emit_event` 带 `target/level/fields`，`VecSubscriber` 收集后按 Span 路径打印。

三、真实用途与 tracing 对照
- Axum / tonic 中间件里 `tracing::info_span!("http_request", method = %method)` 给每个 HTTP/gRPC 请求建 Span。
- `tracing-subscriber` 的 `fmt::layer()` 输出人类可读日志；`EnvFilter` 等价于本课 `Level` 阈值。
- OpenTelemetry 导出时，Span 对应 OTel Span；下一课指标、再一课三信号合一。
资料: https://docs.rs/tracing/latest/tracing/
      https://github.com/tokio-rs/tracing
      https://opentelemetry.io/docs/languages/rust/
*/

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

// ── 级别与字段（对照 tracing::Level / tracing::field）────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Fields(HashMap<&'static str, String>);

impl Fields {
    fn new() -> Self {
        Self(HashMap::new())
    }

    fn with(mut self, k: &'static str, v: impl Into<String>) -> Self {
        self.0.insert(k, v.into());
        self
    }
}

#[derive(Clone, Debug)]
struct LogEvent {
    level: Level,
    target: &'static str,
    message: String,
    fields: Fields,
    span_path: Vec<String>,
}

// ── 直观版：平铺打印 ─────────────────────────────────────────────────

fn log_naive(level: Level, msg: &str) {
  // 仅演示「有级别前缀」；生产里仍缺 span、结构化字段
    println!("[{}] {}", level.as_str(), msg);
}

// ── Span：对照 tracing::span / Span::enter() ─────────────────────────

struct MiniSpan {
    name: String,
    #[allow(dead_code)]
    fields: Fields,
}

struct SpanStack {
    stack: Vec<MiniSpan>,
}

impl SpanStack {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn push(&mut self, name: impl Into<String>, fields: Fields) {
        self.stack.push(MiniSpan {
            name: name.into(),
            fields,
        });
    }

    fn pop(&mut self) {
        self.stack.pop();
    }

    fn path(&self) -> Vec<String> {
        self.stack.iter().map(|s| s.name.clone()).collect()
    }
}

/// 出作用域时在 thread_local 栈上 pop，避免嵌套 Span 时 RefCell 重入借用
struct SpanPopGuard;

impl Drop for SpanPopGuard {
    fn drop(&mut self) {
        TL_STACK.with(|s| s.borrow_mut().pop());
    }
}

// ── Subscriber：对照 tracing::Subscriber trait ─────────────────────────

trait Subscriber {
    fn enabled(&self, level: Level, target: &str) -> bool;
    fn event(&mut self, ev: LogEvent);
}

struct FilteredVecSubscriber {
    max_level: Level,
    allowed_targets: Option<Vec<&'static str>>,
    events: Vec<LogEvent>,
}

impl FilteredVecSubscriber {
    fn new(max_level: Level) -> Self {
        Self {
            max_level,
            allowed_targets: None,
            events: Vec::new(),
        }
    }

    fn only_targets(mut self, targets: Vec<&'static str>) -> Self {
        self.allowed_targets = Some(targets);
        self
    }

    fn drain_events(&mut self) -> Vec<LogEvent> {
        std::mem::take(&mut self.events)
    }

    fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Subscriber for FilteredVecSubscriber {
    fn enabled(&self, level: Level, target: &str) -> bool {
        if level < self.max_level {
            return false;
        }
        if let Some(ref allow) = self.allowed_targets {
            return allow.iter().any(|t| *t == target);
        }
        true
    }

    fn event(&mut self, ev: LogEvent) {
        self.events.push(ev);
    }
}

thread_local! {
    static TL_STACK: RefCell<SpanStack> = RefCell::new(SpanStack::new());
}

#[allow(dead_code)]
fn log_level_error_demo() {
    log_naive(Level::Error, "演示 Error 级别");
}

struct GlobalTracer<S: Subscriber + 'static> {
    subscriber: RefCell<S>,
}

impl<S: Subscriber + 'static> GlobalTracer<S> {
    fn emit(
        &self,
        level: Level,
        target: &'static str,
        message: impl Into<String>,
        fields: Fields,
    ) {
        if !self.subscriber.borrow().enabled(level, target) {
            return;
        }
        let span_path = TL_STACK.with(|s| s.borrow().path());
        let ev = LogEvent {
            level,
            target,
            message: message.into(),
            fields,
            span_path,
        };
        self.subscriber.borrow_mut().event(ev);
    }

    fn with_span<R>(
        &self,
        name: impl Into<String>,
        fields: Fields,
        f: impl FnOnce() -> R,
    ) -> R {
        TL_STACK.with(|s| {
            s.borrow_mut().push(name, fields);
            let _guard = SpanPopGuard;
            f()
        })
    }
}

// ── 业务演示：类似 HTTP handler 调 DB ─────────────────────────────────

fn handle_request<S: Subscriber + 'static>(tracer: &GlobalTracer<S>, user_id: u64) -> bool {
    tracer.with_span(
        "http_request",
        Fields::new()
            .with("method", "GET")
            .with("path", "/api/users"),
        || {
            tracer.emit(
                Level::Info,
                "app::http",
                "收到请求",
                Fields::new().with("user_id", user_id.to_string()),
            );
            tracer.with_span(
                "db_query",
                Fields::new()
                    .with("table", "users")
                    .with("op", "SELECT"),
                || {
                    tracer.emit(
                        Level::Debug,
                        "app::db",
                        "执行查询",
                        Fields::new().with("rows", "1"),
                    );
                    user_id > 0
                },
            )
        },
    )
}

fn format_event_line(ev: &LogEvent) -> String {
    let mut out = String::new();
    let span = if ev.span_path.is_empty() {
        "-".to_string()
    } else {
        ev.span_path.join(" > ")
    };
    let _ = writeln!(
        &mut out,
        "{level:5} target={target} span=[{span}] {msg}",
        level = ev.level.as_str(),
        target = ev.target,
        span = span,
        msg = ev.message
    );
    out
}

fn run_tests() {
    log_naive(Level::Info, "直观版仅一条字符串");

    let tracer = GlobalTracer {
        subscriber: RefCell::new(FilteredVecSubscriber::new(Level::Debug)),
    };
    assert!(handle_request(&tracer, 42));
    let events = tracer.subscriber.borrow_mut().drain_events();
    assert_eq!(events.len(), 2, "应记录 http + db 两条 event");
    assert_eq!(events[0].span_path, vec!["http_request".to_string()]);
    assert_eq!(
        events[1].span_path,
        vec!["http_request".to_string(), "db_query".to_string()]
    );
    assert_eq!(events[0].level, Level::Info);
    assert_eq!(events[1].target, "app::db");

    // 级别过滤：WARN 以上则 DEBUG/INFO 被丢弃
    let tracer2 = GlobalTracer {
        subscriber: RefCell::new(FilteredVecSubscriber::new(Level::Warn)),
    };
    handle_request(&tracer2, 1);
    assert_eq!(tracer2.subscriber.borrow().event_count(), 0);

    tracer2.emit(Level::Warn, "app::http", "慢请求", Fields::new().with("ms", "800"));
    assert_eq!(tracer2.subscriber.borrow().event_count(), 1);

    // target 过滤：对照 EnvFilter 按 crate 模块
    let tracer3 = GlobalTracer {
        subscriber: RefCell::new(
            FilteredVecSubscriber::new(Level::Trace).only_targets(vec!["app::db"]),
        ),
    };
    handle_request(&tracer3, 5);
    let evs = tracer3.subscriber.borrow_mut().drain_events();
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].target, "app::db");
    assert!(!events[0].fields.0.is_empty());

    log_level_error_demo();

    println!("✅ tracing 心智（Event + Span 栈 + Subscriber 过滤）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 tracing 心智 ************\n");

    println!("── 直观版 ──");
    log_naive(Level::Warn, "磁盘余量低");

    println!("\n── 进阶版：Span 嵌套 + 结构化 Event ──");
    let tracer = GlobalTracer {
        subscriber: RefCell::new(FilteredVecSubscriber::new(Level::Trace)),
    };
    handle_request(&tracer, 1001);
    for ev in tracer.subscriber.borrow_mut().drain_events() {
        print!("{}", format_event_line(&ev));
    }

    println!(
        "\n对照生产: tracing::info!(target: \"app::http\", user_id = %id, \"...\")"
    );
    println!("文档: https://docs.rs/tracing/latest/tracing/");
    println!("仓库: https://github.com/tokio-rs/tracing\n");

    run_tests();
}

fn main() {
    bootstrap();
}