/*
[主题]: 弹性心智 —— 指数退避重试 + 熔断器（Closed / Open / HalfOpen）
[难度]: Medium → Hard
[前置]: `07_async_tokio`（超时概念）、`16_production_rust/00_errors`（可重试 vs 致命错误）

一、主题说明
- **重试**：仅对**瞬态**失败（超时、503、连接 reset）；须**指数退避 + 抖动**，并设最大次数。
- **熔断**：连续失败打开电路，快速失败保护下游；冷却后 **半开** 探活。
- **陷阱**：对非幂等 POST 无脑重试；重试风暴；Open 状态仍打满下游。

二、写法对比
1. 直观版: `while` 循环 + `sleep`，无抖动、无错误分类。
2. 进阶版: `RetryPolicy` + `CircuitBreaker` + `CallLabel` 统计，可注入假时钟。

三、真实用途与生产对照
| 本课 | 生产组件 |
| --- | --- |
| `RetryPolicy` | `reqwest` + `backoff` / Tower `Retry` |
| `CircuitBreaker` | `failsafe` / `resilience4j` 心智 / gRPC deadline |
| `is_transient` | sqlx 连接池超时 vs UniqueViolation |
| 抖动 | AWS 架构最佳实践 /避免 herd |

资料: https://docs.rs/tower/latest/tower/retry/struct.Retry.html
      https://sre.google/sre-book/addressing-cascading-failures/
      https://docs.rs/anyhow/latest/anyhow/
*/

use std::fmt;
use std::time::{Duration, Instant};

// ══════════════════════════════════════════════════════════════════
// 错误分类：可否重试
// ══════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
enum DownstreamError {
    Transient(&'static str),
    Fatal(&'static str),
}

impl fmt::Display for DownstreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownstreamError::Transient(m) => write!(f, "瞬态: {m}"),
            DownstreamError::Fatal(m) => write!(f, "致命: {m}"),
        }
    }
}

fn is_transient(err: &DownstreamError) -> bool {
    matches!(err, DownstreamError::Transient(_))
}

// ══════════════════════════════════════════════════════════════════
// 直观版：朴素重试
// ══════════════════════════════════════════════════════════════════

fn call_downstream_naive(attempt: u32) -> Result<&'static str, DownstreamError> {
    if attempt < 2 {
        Err(DownstreamError::Transient("连接重置"))
    } else {
        Ok("payload")
    }
}

fn retry_naive(max: u32) -> Result<&'static str, DownstreamError> {
    // 陷阱：固定间隔、无抖动、不区分 Fatal
    for attempt in 0..max {
        match call_downstream_naive(attempt) {
            Ok(v) => return Ok(v),
            Err(e) if is_transient(&e) && attempt + 1 < max => continue,
            Err(e) => return Err(e),
        }
    }
    Err(DownstreamError::Transient("重试用尽"))
}

// ══════════════════════════════════════════════════════════════════
// 进阶：RetryPolicy（指数退避 + 抖动）
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
struct RetryPolicy {
    max_attempts: u32,
    base_delay: Duration,
    max_delay: Duration,
    /// 确定性「抖动」种子，测试可复现
    jitter_seed: u64,
}

impl RetryPolicy {
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = self.base_delay.as_millis() as u64 * 2u64.saturating_pow(attempt);
        let capped = exp.min(self.max_delay.as_millis() as u64);
        let jitter = (self.jitter_seed.wrapping_mul(1103515245 + attempt as u64)) % 50;
        Duration::from_millis(capped + jitter)
    }

    fn execute<F, T>(&self, mut f: F) -> Result<T, DownstreamError>
    where
        F: FnMut(u32) -> Result<T, DownstreamError>,
    {
        let mut last = DownstreamError::Transient("未调用");
        for attempt in 0..self.max_attempts {
            match f(attempt) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if !is_transient(&e) {
                        return Err(e);
                    }
                    last = e;
                    if attempt + 1 >= self.max_attempts {
                        break;
                    }
                    let _wait = self.delay_for_attempt(attempt);
                    // 课内同步：不真 sleep，避免 verify 变慢；bootstrap 打印计划等待
                }
            }
        }
        Err(last)
    }
}

// ══════════════════════════════════════════════════════════════════
// 熔断器
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
struct CircuitBreaker {
    failure_threshold: u32,
    success_threshold: u32,
    open_duration: Duration,
    consecutive_failures: u32,
    half_open_successes: u32,
    state: CircuitState,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    fn new(failure_threshold: u32, success_threshold: u32, open_duration: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            open_duration,
            consecutive_failures: 0,
            half_open_successes: 0,
            state: CircuitState::Closed,
            opened_at: None,
        }
    }

    fn state(&self) -> CircuitState {
        self.state
    }

    fn allow_call(&mut self, now: Instant) -> Result<(), DownstreamError> {
        if self.state == CircuitState::Open {
            let opened = self.opened_at.expect("Open 必有 opened_at");
            if now.duration_since(opened) >= self.open_duration {
                self.state = CircuitState::HalfOpen;
                self.half_open_successes = 0;
            } else {
                return Err(DownstreamError::Transient("熔断打开 — 快速失败"));
            }
        }
        Ok(())
    }

    fn record_success(&mut self) {
        match self.state {
            CircuitState::HalfOpen => {
                self.half_open_successes += 1;
                if self.half_open_successes >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.consecutive_failures = 0;
                    self.opened_at = None;
                }
            }
            CircuitState::Closed => {
                self.consecutive_failures = 0;
            }
            CircuitState::Open => {}
        }
    }

    fn record_failure(&mut self, now: Instant) {
        self.consecutive_failures += 1;
        if self.state == CircuitState::HalfOpen {
            self.trip_open(now);
            return;
        }
        if self.consecutive_failures >= self.failure_threshold {
            self.trip_open(now);
        }
    }

    fn trip_open(&mut self, now: Instant) {
        self.state = CircuitState::Open;
        self.opened_at = Some(now);
        self.half_open_successes = 0;
    }
}

struct ResilientClient {
    retry: RetryPolicy,
    breaker: CircuitBreaker,
}

impl ResilientClient {
    fn call<F, T>(&mut self, now: Instant, mut f: F) -> Result<T, DownstreamError>
    where
        F: FnMut(u32) -> Result<T, DownstreamError>,
    {
        self.breaker.allow_call(now)?;
        let attempts = self.retry.max_attempts;
        let mut attempt = 0u32;
        let result = loop {
            match f(attempt) {
                Ok(v) => break Ok(v),
                Err(e) if is_transient(&e) && attempt + 1 < attempts => {
                    attempt += 1;
                    continue;
                }
                Err(e) => break Err(e),
            }
        };
        match &result {
            Ok(_) => self.breaker.record_success(),
            Err(_) => self.breaker.record_failure(now),
        }
        result
    }
}

// ══════════════════════════════════════════════════════════════════
// 幂等陷阱
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq)]
enum HttpVerb {
    Get,
    Post,
}

fn may_retry(verb: HttpVerb, err: &DownstreamError) -> bool {
    if !is_transient(err) {
        return false;
    }
    match verb {
        HttpVerb::Get => true,
        HttpVerb::Post => false, // 除非有 Idempotency-Key
    }
}

fn run_tests() {
    assert_eq!(retry_naive(3).unwrap(), "payload", "朴素重试 Eventually");

    let policy = RetryPolicy {
        max_attempts: 4,
        base_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(200),
        jitter_seed: 7,
    };
    let mut calls = 0u32;
    let r = policy.execute(|attempt| {
        calls += 1;
        call_downstream_naive(attempt)
    });
    assert_eq!(r.unwrap(), "payload");
    assert_eq!(calls, 3, "应在第 3 次成功");

    let d0 = policy.delay_for_attempt(0);
    let d1 = policy.delay_for_attempt(1);
    assert!(d1 > d0, "指数退避应递增");

    let fatal: Result<&str, DownstreamError> =
        policy.execute(|_| Err(DownstreamError::Fatal("校验失败")));
    assert_eq!(fatal.unwrap_err(), DownstreamError::Fatal("校验失败"));

    let mut cb = CircuitBreaker::new(2, 1, Duration::from_millis(50));
    let t0 = Instant::now();
    assert!(cb.allow_call(t0).is_ok());
    cb.record_failure(t0);
    assert_eq!(cb.state(), CircuitState::Closed);
    cb.record_failure(t0);
    assert_eq!(cb.state(), CircuitState::Open);
    assert!(cb.allow_call(t0).is_err(), "Open 应拒绝");

    let t1 = t0 + Duration::from_millis(60);
    assert!(cb.allow_call(t1).is_ok(), "冷却后 HalfOpen");
    assert_eq!(cb.state(), CircuitState::HalfOpen);
    cb.record_success();
    assert_eq!(cb.state(), CircuitState::Closed);

    let mut client = ResilientClient {
        retry: policy.clone(),
        breaker: CircuitBreaker::new(5, 1, Duration::from_millis(100)),
    };
    let ok = client.call(t0, |a| call_downstream_naive(a));
    assert!(ok.is_ok());

    assert!(may_retry(HttpVerb::Get, &DownstreamError::Transient("x")));
    assert!(!may_retry(HttpVerb::Post, &DownstreamError::Transient("x")));

    println!("✅ l00_resilience 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 弹性：重试 + 熔断 ************\n");

    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay: Duration::from_millis(100),
        max_delay: Duration::from_secs(2),
        jitter_seed: 42,
    };
    println!(
        "attempt 0 退避 {:?}, attempt 1 退避 {:?}",
        policy.delay_for_attempt(0),
        policy.delay_for_attempt(1)
    );

    match retry_naive(3) {
        Ok(p) => println!("朴素重试成功: {p}"),
        Err(e) => println!("失败: {e}"),
    }

    println!("\n生产对照: Tower Retry Layer + deadline；gRPC 用 per-RPC 重试策略");
    println!("陷阱: POST 支付无 Idempotency-Key 时禁止自动重试");

    run_tests();
}

fn main() {
    bootstrap();
}