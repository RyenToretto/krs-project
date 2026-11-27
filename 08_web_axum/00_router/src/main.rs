/*
[主题]: Router 心智 —— 用 HashMap 路由表模拟 Axum 的「注册 + 派发」
[难度]: Medium
[前置]: 03_error_collections、04_traits_generics

一、主题说明
- HTTP 框架第一件事：**把「方法 + 路径」映射到处理函数**。
- 真实 Axum：`Router::new().route("/health", get(handler)).merge(...)`，内部维护路由树 + `MethodRouter`。
- 本课迷你版：`HashMap<RouteKey, HandlerId>` + 精确路径/简单 `:param` 匹配，**不 bind 端口**，用 `dispatch(&Request)` 自测。

二、写法对比
1. 直观版: `match (method, path) { ... }` 硬编码分支。
2. 进阶版: `MiniRouter` 注册路由、合并子路由、`404` 兜底。

三、真实用途与对照
- Axum `Router` / `MethodRouter` / `nest`：https://docs.rs/axum/latest/axum/struct.Router.html
- 生产里还会做：`fallback`、静态文件、`with_state`（见本阶段后续课）。
资料: https://docs.rs/axum/latest/axum/
      https://github.com/tokio-rs/axum
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;

/// 迷你 HTTP 方法（真实 Axum 用 `http::Method`）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "DELETE" => Some(HttpMethod::Delete),
            _ => None,
        }
    }
}

/// 假请求（真实场景由 hyper/axum 解析）
#[derive(Clone, Debug)]
struct Request {
    method: HttpMethod,
    path: String,
}

/// 假响应
#[derive(Clone, Debug, PartialEq)]
struct Response {
    status: u16,
    body: String,
}

type HandlerFn = fn(&Request, &HashMap<String, String>) -> Response;

/// 路由表里一条「精确路径」记录
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct RouteKey {
    method: HttpMethod,
    path: String,
}

/// 带 `:id` 这类占位符的模式（进阶：模拟 axum 的 `Path<T>` 解析前一步）
#[derive(Clone, Debug)]
struct PatternRoute {
    method: HttpMethod,
    segments: Vec<String>, // 如 ["users", ":id"]
    handler: HandlerId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HandlerId {
    Health,
    Echo,
    UserShow,
    NotFound,
}

fn handler_health(_req: &Request, _params: &HashMap<String, String>) -> Response {
    Response {
        status: 200,
        body: "ok".into(),
    }
}

fn handler_echo(_req: &Request, _params: &HashMap<String, String>) -> Response {
    Response {
        status: 200,
        body: "echo".into(),
    }
}

fn handler_user_show(_req: &Request, params: &HashMap<String, String>) -> Response {
    let id = params.get("id").cloned().unwrap_or_else(|| "?".into());
    Response {
        status: 200,
        body: format!("user:{id}"),
    }
}

fn handler_not_found(_req: &Request, _params: &HashMap<String, String>) -> Response {
    Response {
        status: 404,
        body: "not found".into(),
    }
}

fn resolve_handler(id: HandlerId) -> HandlerFn {
    match id {
        HandlerId::Health => handler_health,
        HandlerId::Echo => handler_echo,
        HandlerId::UserShow => handler_user_show,
        HandlerId::NotFound => handler_not_found,
    }
}

/// 直观版：手写 match，路径一多就难维护（对照：没上 Router 的脚本式 API）
fn dispatch_naive(req: &Request) -> Response {
    match (req.method, req.path.as_str()) {
        (HttpMethod::Get, "/health") => handler_health(req, &HashMap::new()),
        (HttpMethod::Post, "/echo") => handler_echo(req, &HashMap::new()),
        _ => handler_not_found(req, &HashMap::new()),
    }
}

/// 进阶版：迷你 Router（对照 Axum `Router::route` + `nest` + `fallback`）
struct MiniRouter {
    exact: HashMap<RouteKey, HandlerId>,
    patterns: Vec<PatternRoute>,
    fallback: HandlerId,
}

impl MiniRouter {
    fn new() -> Self {
        Self {
            exact: HashMap::new(),
            patterns: Vec::new(),
            fallback: HandlerId::NotFound,
        }
    }

    /// 对照 `.route("/x", get(h))`
    fn route(mut self, method: HttpMethod, path: &str, id: HandlerId) -> Self {
        self.exact.insert(
            RouteKey {
                method,
                path: path.into(),
            },
            id,
        );
        self
    }

    /// 对照 `.route("/users/:id", get(h))` 的简化版
    fn route_pattern(mut self, method: HttpMethod, pattern: &str, id: HandlerId) -> Self {
        let segments: Vec<String> = pattern.split('/').filter(|s| !s.is_empty()).map(String::from).collect();
        self.patterns.push(PatternRoute {
            method,
            segments,
            handler: id,
        });
        self
    }

    /// 对照 `Router::merge` / `nest`：把另一套路由表并进（仅 exact 键，演示即可）
    fn merge(mut self, other: MiniRouter) -> Self {
        self.exact.extend(other.exact);
        self.patterns.extend(other.patterns);
        self
    }

    /// 对照 `.fallback(handler)`
    fn fallback(mut self, id: HandlerId) -> Self {
        self.fallback = id;
        self
    }

    fn dispatch(&self, req: &Request) -> Response {
        let key = RouteKey {
            method: req.method,
            path: req.path.clone(),
        };
        if let Some(id) = self.exact.get(&key) {
            return resolve_handler(*id)(req, &HashMap::new());
        }

        let path_segs: Vec<&str> = req.path.split('/').filter(|s| !s.is_empty()).collect();
        for pat in &self.patterns {
            if pat.method != req.method || pat.segments.len() != path_segs.len() {
                continue;
            }
            let mut params = HashMap::new();
            let mut ok = true;
            for (ps, rs) in pat.segments.iter().zip(path_segs.iter()) {
                if ps.starts_with(':') {
                    params.insert(ps[1..].to_string(), (*rs).to_string());
                } else if ps != rs {
                    ok = false;
                    break;
                }
            }
            if ok {
                return resolve_handler(pat.handler)(req, &params);
            }
        }

        resolve_handler(self.fallback)(req, &HashMap::new())
    }
}

fn run_tests() {
    let req_health = Request {
        method: HttpMethod::Get,
        path: "/health".into(),
    };
    assert_eq!(dispatch_naive(&req_health).body, "ok");

    let api = MiniRouter::new()
        .route(HttpMethod::Get, "/health", HandlerId::Health)
        .route(HttpMethod::Post, "/echo", HandlerId::Echo)
        .route_pattern(HttpMethod::Get, "/users/:id", HandlerId::UserShow)
        .fallback(HandlerId::NotFound);

    let admin = MiniRouter::new().route(HttpMethod::Get, "/admin/ping", HandlerId::Health);
    let app = api.merge(admin);

    assert_eq!(app.dispatch(&req_health).status, 200);
    assert_eq!(
        app.dispatch(&Request {
            method: HttpMethod::Post,
            path: "/echo".into(),
        })
        .body,
        "echo"
    );
    assert_eq!(
        app.dispatch(&Request {
            method: HttpMethod::Get,
            path: "/users/42".into(),
        })
        .body,
        "user:42"
    );
    assert_eq!(
        app.dispatch(&Request {
            method: HttpMethod::Get,
            path: "/nope".into(),
        })
        .status,
        404
    );
    assert_eq!(
        app.dispatch(&Request {
            method: HttpMethod::Get,
            path: "/admin/ping".into(),
        })
        .body,
        "ok"
    );

    assert!(HttpMethod::from_str("GET") == Some(HttpMethod::Get));
    assert!(HttpMethod::from_str("PATCH").is_none());

    println!("✅ Router 心智（迷你 Axum 对照）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ Router 心智 ************\n");
    println!("对照 Axum：Router::route / merge / fallback —— 本课用 HashMap + 模式匹配离线演示。\n");

    let app = MiniRouter::new()
        .route(HttpMethod::Get, "/health", HandlerId::Health)
        .route_pattern(HttpMethod::Get, "/users/:id", HandlerId::UserShow);

    for path in ["/health", "/users/7", "/missing"] {
        let resp = app.dispatch(&Request {
            method: HttpMethod::Get,
            path: path.into(),
        });
        println!("GET {path} -> {} {}", resp.status, resp.body);
    }

    run_tests();
}

fn main() {
    bootstrap();
}