/*
[主题]: 健康检查 —— liveness 与 readiness 分离（离线模拟 /livez /readyz）
[难度]: Medium → Hard
[前置]: `08_web_axum/00_router`（路由）、`15_project_capstone`（/health）

一、主题说明
- **Liveness**：进程是否**死锁 / 僵死**；失败 → 编排器**重启**容器（慎用重量级检查）。
- **Readiness**：是否**可接流量**（DB、缓存预热、依赖就绪）；失败 → 从 Service 端点摘除，**不**一定重启。
- **陷阱**：把慢 SQL 放进 liveness → 雪崩重启；readiness 过宽 → 流量打到未预热实例。

二、写法对比
1. 直观版: 单一 `/health` 返回 200，liveness/readiness 混为一谈。
2. 进阶版: `ProbeKind` + `HealthRegistry`，分别聚合探针并映射 HTTP 状态码。

三、真实用途与生产对照
| 本课 | K8s / 负载均衡 |
| --- | --- |
| `GET /livez` | `livenessProbe.httpGet` |
| `GET /readyz` | `readinessProbe.httpGet` |
| `ProbeStatus::Unhealthy` on readiness | Endpoints 摘除 Pod |
| `ProbeStatus::Unhealthy` on liveness | kubelet 重启容器 |

资料:
https://doc.rust-lang.org/book/
https://docs.rs/tracing/latest/tracing/
*/

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

// ══════════════════════════════════════════════════════════════════
// HTTP 迷你模型（与 08_web_axum 课一致风格，无第三方框架）
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Eq)]
struct HttpRequest {
    method: &'static str,
    path: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HttpResponse {
    status: u16,
    body: String,
}

// ══════════════════════════════════════════════════════════════════
// 探针类型与状态
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeKind {
    Liveness,
    Readiness,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProbeStatus {
    Healthy,
    Unhealthy { reason: &'static str },
}

impl ProbeStatus {
    fn is_ok(&self) -> bool {
        matches!(self, ProbeStatus::Healthy)
    }
}

impl fmt::Display for ProbeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProbeStatus::Healthy => write!(f, "ok"),
            ProbeStatus::Unhealthy { reason } => write!(f, "fail: {reason}"),
        }
    }
}

/// 探针 trait：对照 `tower` 或 axum 里拆分的 check 函数
trait HealthProbe: fmt::Debug {
    fn name(&self) -> &'static str;
    fn kind(&self) -> ProbeKind;
    fn check(&self) -> ProbeStatus;
}

// ══════════════════════════════════════════════════════════════════
// 应用依赖态（模拟 DB / 预热）
// ══════════════════════════════════════════════════════════════════

struct AppState {
    started_at: Instant,
    db_connected: bool,
    cache_warmed: bool,
    /// 模拟死锁：liveness 应失败
    deadlock_sim: bool,
}

impl AppState {
    fn new() -> Self {
        Self {
            started_at: Instant::now(),
            db_connected: false,
            cache_warmed: false,
            deadlock_sim: false,
        }
    }

    fn warm_up(&mut self) {
        self.db_connected = true;
        self.cache_warmed = true;
    }
}

/// Liveness：只关心**存活**，不做外部 IO（对照 K8s 文档建议）
#[derive(Debug)]
struct ProcessAliveProbe {
    deadlock: AtomicBool,
}

impl ProcessAliveProbe {
    fn new() -> Self {
        Self {
            deadlock: AtomicBool::new(false),
        }
    }
}

impl HealthProbe for ProcessAliveProbe {
    fn name(&self) -> &'static str {
        "process_alive"
    }

    fn kind(&self) -> ProbeKind {
        ProbeKind::Liveness
    }

    fn check(&self) -> ProbeStatus {
        if self.deadlock.load(Ordering::Relaxed) {
            ProbeStatus::Unhealthy {
                reason: "事件循环未推进（死锁模拟）",
            }
        } else {
            ProbeStatus::Healthy
        }
    }
}

/// Readiness：依赖就绪（对照 startupProbe + readiness 组合）
struct DependencyReadyProbe<'a> {
    state: &'a AppState,
    min_warmup: Duration,
}

impl fmt::Debug for DependencyReadyProbe<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependencyReadyProbe")
            .field("min_warmup", &self.min_warmup)
            .finish()
    }
}

impl HealthProbe for DependencyReadyProbe<'_> {
    fn name(&self) -> &'static str {
        "dependency_ready"
    }

    fn kind(&self) -> ProbeKind {
        ProbeKind::Readiness
    }

    fn check(&self) -> ProbeStatus {
        if self.state.deadlock_sim {
            return ProbeStatus::Unhealthy {
                reason: "进程僵死（readiness 也应失败）",
            };
        }
        if self.state.started_at.elapsed() < self.min_warmup {
            return ProbeStatus::Unhealthy {
                reason: "预热未完成",
            };
        }
        if !self.state.db_connected {
            return ProbeStatus::Unhealthy {
                reason: "数据库未连接",
            };
        }
        if !self.state.cache_warmed {
            return ProbeStatus::Unhealthy {
                reason: "缓存未预热",
            };
        }
        ProbeStatus::Healthy
    }
}

// ══════════════════════════════════════════════════════════════════
// 路由器（按探针种类即时求值，避免自引用生命周期）
// ══════════════════════════════════════════════════════════════════

struct HealthRouter<'a> {
    state: &'a AppState,
    alive: &'a ProcessAliveProbe,
    min_warmup: Duration,
}

impl<'a> HealthRouter<'a> {
    fn new(state: &'a AppState, alive: &'a ProcessAliveProbe, min_warmup: Duration) -> Self {
        Self {
            state,
            alive,
            min_warmup,
        }
    }

    fn evaluate(&self, kind: ProbeKind) -> ProbeStatus {
        let probes: Vec<ProbeStatus> = match kind {
            ProbeKind::Liveness => vec![self.alive.check()],
            ProbeKind::Readiness => vec![DependencyReadyProbe {
                state: self.state,
                min_warmup: self.min_warmup,
            }
            .check()],
        };
        for st in probes {
            if !st.is_ok() {
                return st;
            }
        }
        ProbeStatus::Healthy
    }

    fn handle(&self, req: &HttpRequest) -> HttpResponse {
        if req.method != "GET" {
            return HttpResponse {
                status: 405,
                body: "method not allowed".into(),
            };
        }
        let (kind, path) = match req.path.as_str() {
            "/livez" => (ProbeKind::Liveness, "livez"),
            "/readyz" => (ProbeKind::Readiness, "readyz"),
            "/health" => {
                let live = self.evaluate(ProbeKind::Liveness);
                let ready = self.evaluate(ProbeKind::Readiness);
                if live.is_ok() && ready.is_ok() {
                    return HttpResponse {
                        status: 200,
                        body: "ok".into(),
                    };
                }
                return HttpResponse {
                    status: 503,
                    body: format!("live={live} ready={ready}"),
                };
            }
            _ => {
                return HttpResponse {
                    status: 404,
                    body: "not found".into(),
                };
            }
        };
        let st = self.evaluate(kind);
        if st.is_ok() {
            HttpResponse {
                status: 200,
                body: path.into(),
            }
        } else {
            HttpResponse {
                status: 503,
                body: st.to_string(),
            }
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// 直观版：单端点永远 200
// ══════════════════════════════════════════════════════════════════

fn naive_health(_state: &AppState) -> HttpResponse {
    // 陷阱：DB 未连也返回 200 → 流量打进半残实例
    HttpResponse {
        status: 200,
        body: "ok".into(),
    }
}

// ══════════════════════════════════════════════════════════════════
// 测试
// ══════════════════════════════════════════════════════════════════

fn run_tests() {
    let live_req = HttpRequest {
        method: "GET",
        path: "/livez".into(),
    };
    let ready_req = HttpRequest {
        method: "GET",
        path: "/readyz".into(),
    };

    let alive = ProcessAliveProbe::new();

    {
        let state = AppState::new();
        let router = HealthRouter::new(&state, &alive, Duration::from_millis(0));
        assert_eq!(router.handle(&live_req).status, 200);
        assert_eq!(
            router.handle(&ready_req).status,
            503,
            "未预热时 readiness 应失败"
        );
    }

    {
        let mut state = AppState::new();
        state.warm_up();
        let router = HealthRouter::new(&state, &alive, Duration::from_millis(0));
        assert_eq!(router.handle(&ready_req).status, 200);
    }

    {
        let mut state = AppState::new();
        state.warm_up();
        state.db_connected = false;
        let router = HealthRouter::new(&state, &alive, Duration::from_millis(0));
        assert_eq!(
            router.handle(&live_req).status,
            200,
            "DB 失败不应杀 liveness"
        );
        assert_eq!(router.handle(&ready_req).status, 503);
    }

    {
        let mut state = AppState::new();
        state.warm_up();
        alive.deadlock.store(true, Ordering::Relaxed);
        let router = HealthRouter::new(&state, &alive, Duration::from_millis(0));
        assert_eq!(
            router.handle(&live_req).status,
            503,
            "死锁应使 liveness 失败 → 编排器重启"
        );
        alive.deadlock.store(false, Ordering::Relaxed);
    }

    let state = AppState::new();
    let bad = naive_health(&state);
    assert_eq!(bad.status, 200, "反模式演示：永远 200");

    println!("✅ l00_health_checks 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 健康检查：/livez vs /readyz ************\n");

    let mut state = AppState::new();
    let alive = ProcessAliveProbe::new();
    println!(
        "探针注册: {} ({:?}) + {} ({:?})",
        alive.name(),
        alive.kind(),
        DependencyReadyProbe {
            state: &state,
            min_warmup: Duration::from_millis(50),
        }
        .name(),
        ProbeKind::Readiness,
    );

    println!("启动瞬间:");
    {
        let router = HealthRouter::new(&state, &alive, Duration::from_millis(50));
        for path in ["/livez", "/readyz"] {
            let resp = router.handle(&HttpRequest {
                method: "GET",
                path: path.into(),
            });
            println!("  {path} -> {} {}", resp.status, resp.body);
        }
    }

    state.warm_up();
    println!("\n预热后:");
    {
        let router = HealthRouter::new(&state, &alive, Duration::from_millis(0));
        for path in ["/livez", "/readyz", "/health"] {
            let resp = router.handle(&HttpRequest {
                method: "GET",
                path: path.into(),
            });
            println!("  {path} -> {} {}", resp.status, resp.body);
        }
    }

    println!("\n陷阱回顾:");
    println!("- liveness 别做慢查询 / 跨网依赖");
    println!("- readiness 失败只摘流量，别误用 liveness 探针路径");
    println!("- 结构化日志可用 tracing 记录 probe 失败原因（勿打秘密）");

    run_tests();
}

fn main() {
    bootstrap();
}