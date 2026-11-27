/*
[主题]: OpenTelemetry 心智 —— Trace 上下文传播与三信号合一（纯 Rust 迷你实现）
[难度]: Medium-Hard
[前置]: 本阶段 `00_tracing`、`01_metrics`；`12_grpc_tonic/00_split`（跨服务调用）

一、主题说明
- **OpenTelemetry（OTel）** 统一 **Traces / Metrics / Logs** 语义，便于换后端（Jaeger、Tempo、Prometheus、OTLP Collector）。
- **TraceId** 标识一次分布式请求；**SpanId** 标识链路上一个工作单元；**ParentSpanId** 连起树。
- **W3C Trace Context**：HTTP 头 `traceparent: 00-{trace-id}-{parent-span-id}-{flags}` 在微服务间传递（对照 tonic/axum 中间件注入）。
- Rust 常用 `opentelemetry` + `tracing-opentelemetry` 把 tracing Span 导出为 OTel；本课手写 Propagator + InMemoryExporter。

二、写法对比
1. 直观版: 每个服务自己 `println` 请求 id —— 无法跨进程关联。
2. 进阶版: `TraceContext` 解析/注入 `traceparent`，`OtelPipeline` 一次请求同时写 Span、日志字段、Counter。

三、真实用途
- K8s 里 sidecar Collector 收 OTLP，Grafana 上 trace 串起 gRPC + DB + 缓存。
- 与 tracing 的关系：`tracing` 负责埋点 API，OTel SDK 负责导出与跨语言互操作。
资料: https://opentelemetry.io/docs/languages/rust/
      https://docs.rs/tracing/latest/tracing/
      https://github.com/tokio-rs/tracing
*/

use std::collections::HashMap;

// ── 128-bit TraceId、64-bit SpanId（十六进制字符串）────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
struct TraceId([u8; 16]);

impl TraceId {
    fn new_random(seed: u64) -> Self {
        let mut buf = [0u8; 16];
        buf[..8].copy_from_slice(&seed.to_be_bytes());
        buf[8..].copy_from_slice(&(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)).to_be_bytes());
        Self(buf)
    }

    fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 32 {
            return None;
        }
        let mut buf = [0u8; 16];
        for i in 0..16 {
            let pair = &s[i * 2..i * 2 + 2];
            buf[i] = u8::from_str_radix(pair, 16).ok()?;
        }
        Some(Self(buf))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpanId([u8; 8]);

impl SpanId {
    fn new_random(n: u64) -> Self {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&n.to_be_bytes());
        Self(buf)
    }

    fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn from_hex(s: &str) -> Option<Self> {
        if s.len() != 16 {
            return None;
        }
        let mut buf = [0u8; 8];
        for i in 0..8 {
            let pair = &s[i * 2..i * 2 + 2];
            buf[i] = u8::from_str_radix(pair, 16).ok()?;
        }
        Some(Self(buf))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TraceFlags(u8);

impl TraceFlags {
    const SAMPLED: Self = Self(0x01);

    fn is_sampled(self) -> bool {
        self.0 & 0x01 != 0
    }

    fn to_hex(self) -> String {
        format!("{:02x}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TraceContext {
    trace_id: TraceId,
    span_id: SpanId,
    parent_span_id: Option<SpanId>,
    flags: TraceFlags,
}

// ── W3C traceparent（版本 00）────────────────────────────────────────

fn format_traceparent(ctx: &TraceContext) -> String {
    format!(
        "00-{}-{}-{}",
        ctx.trace_id.to_hex(),
        ctx.span_id.to_hex(),
        ctx.flags.to_hex()
    )
}

fn parse_traceparent(header: &str) -> Option<TraceContext> {
    let parts: Vec<&str> = header.split('-').collect();
    if parts.len() != 4 || parts[0] != "00" {
        return None;
    }
    let trace_id = TraceId::from_hex(parts[1])?;
    let span_id = SpanId::from_hex(parts[2])?;
    let flags_byte = u8::from_str_radix(parts[3], 16).ok()?;
    Some(TraceContext {
        trace_id,
        span_id,
        parent_span_id: None,
        flags: TraceFlags(flags_byte),
    })
}

// ── 直观版：各服务各打各的 id ─────────────────────────────────────────

fn log_naive_service_a(local_id: u64) {
    println!("[service_a] local_id={}（无法与 B 关联）", local_id);
}

fn log_naive_service_b(another_id: u64) {
    println!("[service_b] another_id={}", another_id);
}

// ── OTel Span 记录 + 内存导出 ─────────────────────────────────────────

#[derive(Clone, Debug)]
struct FinishedSpan {
    name: &'static str,
    trace_id: TraceId,
    span_id: SpanId,
    parent: Option<SpanId>,
    attributes: HashMap<&'static str, String>,
    duration_ms: u64,
}

struct InMemorySpanExporter {
    spans: Vec<FinishedSpan>,
}

impl InMemorySpanExporter {
    fn new() -> Self {
        Self { spans: Vec::new() }
    }

    fn export(&mut self, span: FinishedSpan) {
        self.spans.push(span);
    }

    fn drain(&mut self) -> Vec<FinishedSpan> {
        std::mem::take(&mut self.spans)
    }
}

struct SimpleCounter {
    value: u64,
}

impl SimpleCounter {
    fn inc(&mut self) {
        self.value += 1;
    }
}

/// 迷你管线：一次「入站请求」建 Span、打结构化日志行、递增指标
struct OtelPipeline {
    exporter: InMemorySpanExporter,
    requests_total: SimpleCounter,
    next_span_seq: u64,
}

impl OtelPipeline {
    fn new() -> Self {
        Self {
            exporter: InMemorySpanExporter::new(),
            requests_total: SimpleCounter { value: 0 },
            next_span_seq: 1,
        }
    }

    fn child_span_id(&mut self) -> SpanId {
        let id = SpanId::new_random(self.next_span_seq);
        self.next_span_seq += 1;
        id
    }

    /// 模拟网关：从 traceparent 恢复上下文，或新建 trace
    fn handle_ingress(
        &mut self,
        service: &'static str,
        traceparent: Option<&str>,
    ) -> (TraceContext, String) {
        let ctx = if let Some(tp) = traceparent {
            parse_traceparent(tp).expect("合法 traceparent")
        } else {
            let tid = TraceId::new_random(0xC0FFEE);
            let sid = self.child_span_id();
            TraceContext {
                trace_id: tid,
                span_id: sid,
                parent_span_id: None,
                flags: TraceFlags::SAMPLED,
            }
        };

        let child_id = self.child_span_id();
        let active = TraceContext {
            trace_id: ctx.trace_id.clone(),
            span_id: child_id,
            parent_span_id: Some(ctx.span_id),
            flags: ctx.flags,
        };

        self.requests_total.inc();
        let log_line = format!(
            "LOG service={} trace_id={} span_id={} parent={:?}",
            service,
            active.trace_id.to_hex(),
            active.span_id.to_hex(),
            active.parent_span_id.as_ref().map(SpanId::to_hex)
        );

        self.exporter.export(FinishedSpan {
            name: service,
            trace_id: active.trace_id.clone(),
            span_id: active.span_id.clone(),
            parent: active.parent_span_id.clone(),
            attributes: HashMap::from([("otel.signal", "trace".into())]),
            duration_ms: 3,
        });

        (active, log_line)
    }

    fn downstream_header(&self, ctx: &TraceContext) -> String {
        format_traceparent(ctx)
    }
}

fn run_tests() {
    log_naive_service_a(1);
    log_naive_service_b(2);

    let mut pipe = OtelPipeline::new();
    let (root_ctx, _) = pipe.handle_ingress("api_gateway", None);
    assert!(root_ctx.flags.is_sampled());
    assert_eq!(pipe.requests_total.value, 1);

    let header = pipe.downstream_header(&root_ctx);
    assert!(header.starts_with("00-"));
    let parsed = parse_traceparent(&header).expect("round-trip");
    assert_eq!(parsed.trace_id.to_hex(), root_ctx.trace_id.to_hex());

    // 下游 B 带着 traceparent 继续同一 trace_id
    let mut pipe_b = OtelPipeline::new();
    let (b_ctx, log_b) = pipe_b.handle_ingress("user_service", Some(&header));
    assert_eq!(b_ctx.trace_id.to_hex(), root_ctx.trace_id.to_hex());
    assert!(log_b.contains("user_service"));

    let spans = pipe_b.exporter.drain();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "user_service");
    assert!(spans[0].parent.is_some());

    // 非法头
    assert!(parse_traceparent("bad").is_none());
    assert!(parse_traceparent("00-tooshort-ffffffffffffffff-01").is_none());

    println!("✅ OTel 心智（TraceContext + traceparent + 三信号管线）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L02 OTel 心智 ************\n");

    println!("── 直观版：各服务本地 id ──");
    log_naive_service_a(99);
    log_naive_service_b(100);

    println!("\n── 进阶版：traceparent 跨服务 ──");
    let mut gateway = OtelPipeline::new();
    let (ctx, log_a) = gateway.handle_ingress("api_gateway", None);
    println!("{}", log_a);
    let tp = gateway.downstream_header(&ctx);
    println!("注入下游头 traceparent = {}", tp);

    let mut user = OtelPipeline::new();
    let (_, log_b) = user.handle_ingress("user_service", Some(&tp));
    println!("{}", log_b);
    for s in user.exporter.drain() {
        println!(
            "导出 Span: name={} trace={} span={} duration_ms={} attrs={:?}",
            s.name,
            s.trace_id.to_hex(),
            s.span_id.to_hex(),
            s.duration_ms,
            s.attributes
        );
    }

    println!("\n对照生产: tracing-opentelemetry Layer + OTLP exporter");
    println!("文档: https://opentelemetry.io/docs/languages/rust/");
    println!("tracing 对接: https://docs.rs/tracing/latest/tracing/\n");

    run_tests();
}

fn main() {
    bootstrap();
}