/*
[主题]: API 网关心智 —— 对外 HTTP 形态、向内 gRPC、聚合与中间件（离线）
[难度]: Medium → Hard
[前置]: `08_web_axum`（Router/中间件）、`12_grpc_tonic/01_tonic`（Stub/Server）

一、主题说明
- **API Gateway**：集群对外通常只有一个入口（REST/GraphQL/OpenAPI）；网关负责**路由、鉴权、限流、协议转换**，再调后端 **gRPC 微服务**。
- 本课 `MiniGateway` 接收 `GatewayHttpRequest`（假装 Axum 已解析），转成 `full_method + payload` 调内存 `BackendPool`。
- **BFF 聚合**：`GET /dashboard/:user_id` 一次调 `GetUser` + `ListOrders`，减少前端往返（注意超时与部分失败策略）。
- 真栈常见：Kong/Envoy + gRPC upstream，或 Axum 网关 + tonic Client。

二、写法对比
1. 直观版: 前端直连各微服务端口，CORS/鉴权散落各处。
2. 进阶版: 网关统一 `Authorization` 校验 + 路由表 + 后端连接池；业务服务不暴露公网。

三、真实用途与对照
- `GatewayLayer::auth` ↔ Axum `.layer(ValidateTokenLayer)` + tonic `Interceptor`
- `route_http` ↔ axum `Router::route` 里 `state.grpc.clone()` 调 `UserServiceClient`
资料: https://docs.rs/tonic/latest/tonic/
      https://github.com/hyperium/tonic
      https://docs.rs/tower/latest/tower/
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;
use std::fmt;

// 复用 01_tonic 同构的迷你 gRPC 片段（单文件自包含，便于离线 verify）

#[derive(Clone, Debug, PartialEq, Eq)]
struct GetUserResponse {
    user_id: u64,
    email: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OrderSummary {
    id: u64,
    product: String,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GrpcCode {
    Ok,
    NotFound,
    Unauthenticated,
    ResourceExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RpcStatus {
    code: GrpcCode,
    message: String,
}

#[allow(dead_code)]
impl RpcStatus {
    fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: GrpcCode::NotFound,
            message: msg.into(),
        }
    }

    fn unauthenticated(msg: impl Into<String>) -> Self {
        Self {
            code: GrpcCode::Unauthenticated,
            message: msg.into(),
        }
    }
}

impl fmt::Display for RpcStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

struct UserBackend {
    users: HashMap<u64, String>,
}

impl UserBackend {
    fn get_user(&self, id: u64) -> Result<GetUserResponse, RpcStatus> {
        self.users
            .get(&id)
            .map(|email| GetUserResponse {
                user_id: id,
                email: email.clone(),
            })
            .ok_or_else(|| RpcStatus::not_found(format!("user {}", id)))
    }
}

struct OrderBackend {
    orders: HashMap<u64, Vec<OrderSummary>>,
}

impl OrderBackend {
    fn list_orders(&self, user_id: u64) -> Vec<OrderSummary> {
        self.orders.get(&user_id).cloned().unwrap_or_default()
    }
}

/// 假装「多个 tonic Server」在一个进程里，网关通过 pool 选后端
struct BackendPool {
    user: UserBackend,
    order: OrderBackend,
}

impl BackendPool {
    fn demo_filled() -> Self {
        let mut users = HashMap::new();
        users.insert(1, "gateway-demo@krs.dev".into());
        let mut orders = HashMap::new();
        orders.insert(
            1,
            vec![
                OrderSummary {
                    id: 100,
                    product: "grpc-book".into(),
                },
                OrderSummary {
                    id: 101,
                    product: "tonic-sticker".into(),
                },
            ],
        );
        Self {
            user: UserBackend { users },
            order: OrderBackend { orders },
        }
    }

    fn grpc_get_user(&self, user_id: u64) -> Result<GetUserResponse, RpcStatus> {
        self.user.get_user(user_id)
    }

    fn grpc_list_orders(&self, user_id: u64) -> Vec<OrderSummary> {
        self.order.list_orders(user_id)
    }
}

// ── HTTP 形态（网关外侧）──────────────────────────────────────────

#[derive(Clone, Debug)]
struct GatewayHttpRequest {
    method: &'static str,
    path: String,
    auth_bearer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GatewayHttpResponse {
    status: u16,
    body: String,
}

// ── 网关中间件状态 ─────────────────────────────────────────────────

struct GatewayState {
    pool: BackendPool,
    /// 简易限流：本分钟计数（生产用 token bucket + Redis）
    request_count: u32,
    request_limit: u32,
    valid_tokens: HashMap<String, u64>, // token -> user_id
}

impl GatewayState {
    fn new(pool: BackendPool) -> Self {
        let mut valid_tokens = HashMap::new();
        valid_tokens.insert("dev-token-abc".into(), 1);
        Self {
            pool,
            request_count: 0,
            request_limit: 100,
            valid_tokens,
        }
    }

    fn authenticate(&self, bearer: Option<&str>) -> Result<u64, GatewayHttpResponse> {
        let token = bearer.unwrap_or("");
        match self.valid_tokens.get(token) {
            Some(uid) => Ok(*uid),
            None => Err(GatewayHttpResponse {
                status: 401,
                body: "missing or invalid bearer".into(),
            }),
        }
    }

    fn check_rate_limit(&mut self) -> Result<(), GatewayHttpResponse> {
        self.request_count += 1;
        if self.request_count > self.request_limit {
            return Err(GatewayHttpResponse {
                status: 429,
                body: "rate limit exceeded".into(),
            });
        }
        Ok(())
    }
}

struct MiniGateway {
    state: GatewayState,
}

impl MiniGateway {
    fn new(state: GatewayState) -> Self {
        Self { state }
    }

    /// 进阶：统一入口，类似 axum Router fallback 前的 dispatch
    fn handle(&mut self, req: GatewayHttpRequest) -> GatewayHttpResponse {
        if let Err(resp) = self.state.check_rate_limit() {
            return resp;
        }

        if req.path.starts_with("/api/") {
            let token = req.auth_bearer.as_deref();
            if let Err(resp) = self.state.authenticate(token) {
                return resp;
            }
        }

        match (req.method, req.path.as_str()) {
            ("GET", "/health") => GatewayHttpResponse {
                status: 200,
                body: "ok".into(),
            },
            ("GET", path) if path.starts_with("/api/users/") => {
                let id_str = path.trim_start_matches("/api/users/");
                let user_id: u64 = match id_str.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return GatewayHttpResponse {
                            status: 400,
                            body: "invalid user id".into(),
                        }
                    }
                };
                match self.state.pool.grpc_get_user(user_id) {
                    Ok(u) => GatewayHttpResponse {
                        status: 200,
                        body: format!(
                            "{{\"user_id\":{},\"email\":\"{}\"}}",
                            u.user_id, u.email
                        ),
                    },
                    Err(s) => GatewayHttpResponse {
                        status: 404,
                        body: format!("{{\"error\":\"{}\"}}", s),
                    },
                }
            }
            ("GET", path) if path.starts_with("/api/dashboard/") => {
                let id_str = path.trim_start_matches("/api/dashboard/");
                let user_id: u64 = match id_str.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        return GatewayHttpResponse {
                            status: 400,
                            body: "invalid user id".into(),
                        }
                    }
                };
                self.aggregate_dashboard(user_id)
            }
            _ => GatewayHttpResponse {
                status: 404,
                body: "route not found".into(),
            },
        }
    }

    fn aggregate_dashboard(&self, user_id: u64) -> GatewayHttpResponse {
        let user = match self.state.pool.grpc_get_user(user_id) {
            Ok(u) => u,
            Err(s) => {
                return GatewayHttpResponse {
                    status: 404,
                    body: format!("{{\"error\":\"{}\"}}", s),
                }
            }
        };
        let orders = self.state.pool.grpc_list_orders(user_id);
        let order_part: String = orders
            .iter()
            .map(|o| format!("{{\"id\":{},\"product\":\"{}\"}}", o.id, o.product))
            .collect::<Vec<_>>()
            .join(",");
        GatewayHttpResponse {
            status: 200,
            body: format!(
                "{{\"user_id\":{},\"email\":\"{}\",\"orders\":[{}]}}",
                user.user_id, user.email, order_part
            ),
        }
    }
}

// ── 直观版：无网关，调用方自己拼 URL ───────────────────────────────

struct NaiveClient;

impl NaiveClient {
    fn call_user_service_direct(user_id: u64) -> String {
        format!("直连 user-service:8081/GetUser?id={}", user_id)
    }

    fn call_order_service_direct(user_id: u64) -> String {
        format!("直连 order-service:8082/ListOrders?user={}", user_id)
    }
}

fn run_tests() {
    assert!(NaiveClient::call_user_service_direct(1).contains("8081"));

    let mut gw = MiniGateway::new(GatewayState::new(BackendPool::demo_filled()));

    let health = gw.handle(GatewayHttpRequest {
        method: "GET",
        path: "/health".into(),
        auth_bearer: None,
    });
    assert_eq!(health.status, 200);

    let no_auth = gw.handle(GatewayHttpRequest {
        method: "GET",
        path: "/api/users/1".into(),
        auth_bearer: None,
    });
    assert_eq!(no_auth.status, 401);

    let user = gw.handle(GatewayHttpRequest {
        method: "GET",
        path: "/api/users/1".into(),
        auth_bearer: Some("dev-token-abc".into()),
    });
    assert_eq!(user.status, 200);
    assert!(user.body.contains("gateway-demo@krs.dev"));

    let dash = gw.handle(GatewayHttpRequest {
        method: "GET",
        path: "/api/dashboard/1".into(),
        auth_bearer: Some("dev-token-abc".into()),
    });
    assert_eq!(dash.status, 200);
    assert!(dash.body.contains("grpc-book"));
    assert!(dash.body.contains("tonic-sticker"));

    let mut tiny_limit = GatewayState::new(BackendPool::demo_filled());
    tiny_limit.request_limit = 2;
    let mut gw2 = MiniGateway::new(tiny_limit);
    assert_eq!(
        gw2
            .handle(GatewayHttpRequest {
                method: "GET",
                path: "/health".into(),
                auth_bearer: None,
            })
            .status,
        200
    );
    assert_eq!(
        gw2
            .handle(GatewayHttpRequest {
                method: "GET",
                path: "/health".into(),
                auth_bearer: None,
            })
            .status,
        200
    );
    let limited = gw2.handle(GatewayHttpRequest {
        method: "GET",
        path: "/health".into(),
        auth_bearer: None,
    });
    assert_eq!(limited.status, 429);

    println!("✅ API 网关心智（鉴权、限流、HTTP→gRPC、聚合）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 API 网关心智 ************\n");

    println!(
        "直观版: {} + {}",
        NaiveClient::call_user_service_direct(1),
        NaiveClient::call_order_service_direct(1)
    );

    let mut gw = MiniGateway::new(GatewayState::new(BackendPool::demo_filled()));
    let resp = gw.handle(GatewayHttpRequest {
        method: "GET",
        path: "/api/dashboard/1".into(),
        auth_bearer: Some("dev-token-abc".into()),
    });
    println!("进阶版聚合 dashboard: status={} body={}", resp.status, resp.body);

    println!("\n对照生产: Axum 网关 handler 内 `let mut client = UserServiceClient::new(channel);`");
    println!("Tower 中间件链: https://docs.rs/tower/latest/tower/\n");

    run_tests();
}

fn main() {
    bootstrap();
}