/*
[主题]: Tower 中间件心智 —— 用函数包装 handler，像洋葱一样嵌套
[难度]: Medium
[前置]: 本阶段 `00_router`、`04_traits_generics`

一、主题说明
- Axum 站在 Tower 上：`Service` 请求进、响应出；**middleware 改的是「内层 Service」**。
- 常见：`TraceLayer` 打日志、`TimeoutLayer`、`CorsLayer`；写法是 `.layer(...)` 叠在 Router 上。
- 本课：`type Handler = fn(Request) -> Response`，`Middleware` 是 `Fn(Handler) -> Handler` 的高阶函数，演示 **洋葱模型**（先进后出）。

二、写法对比
1. 直观版: 在 handler 开头结尾手写 `println!`。
2. 进阶版: `with_middleware(handler, &[log, auth, timer])` 统一组装。

三、真实用途与对照
- Tower Service/Layer：https://docs.rs/tower/latest/tower/
- Axum middleware：https://docs.rs/axum/latest/axum/middleware/index.html
资料: https://docs.rs/tower/latest/tower/
      https://docs.rs/axum/latest/axum/middleware/index.html
      https://www.youtube.com/@RustVideos
*/

use std::cell::RefCell;

#[derive(Clone, Debug, PartialEq)]
struct Request {
    path: String,
    auth_token: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct Response {
    status: u16,
    body: String,
}

/// 对照 `Service<Request, Response>`：用 `Box<dyn Fn>` 便于闭包捕获内层
type Handler = Box<dyn Fn(Request) -> Response>;
type Middleware = fn(Handler) -> Handler;

fn api_handler() -> Handler {
    Box::new(|req: Request| Response {
        status: 200,
        body: format!("data for {}", req.path),
    })
}

/// 对照 Tower Layer：包装内层，可在前后做副作用
fn log_middleware(inner: Handler) -> Handler {
    Box::new(move |req: Request| {
        println!("[log] {} 进入", req.path);
        let resp = inner(req);
        println!("[log] 响应 status={}", resp.status);
        resp
    })
}

fn auth_middleware(inner: Handler) -> Handler {
    Box::new(move |req: Request| match req.auth_token.as_deref() {
        Some("secret") => inner(req),
        _ => Response {
            status: 401,
            body: "unauthorized".into(),
        },
    })
}

/// 用 thread_local 记录「逻辑耗时步数」（无真实计时，避免 sleep）
fn count_steps_middleware(inner: Handler) -> Handler {
    Box::new(move |req: Request| {
        STEP_COUNTER.with(|c| *c.borrow_mut() += 1);
        let resp = inner(req);
        STEP_COUNTER.with(|c| *c.borrow_mut() += 1);
        resp
    })
}

thread_local! {
    static STEP_COUNTER: RefCell<u32> = const { RefCell::new(0) };
}

/// 直观版：逻辑全挤在一个函数里
fn handle_naive(req: Request) -> Response {
    if req.auth_token.as_deref() != Some("secret") {
        return Response {
            status: 401,
            body: "unauthorized".into(),
        };
    }
    Response {
        status: 200,
        body: format!("data for {}", req.path),
    }
}

/// 进阶版：按顺序折叠 middleware（第一个最外层 —— 对照 `.layer` 从下往上包）
fn compose_middleware(mut handler: Handler, layers: &[Middleware]) -> Handler {
    for &mw in layers.iter().rev() {
        handler = mw(handler);
    }
    handler
}

/// 对照 `Router::layer` 链：返回最终可调用的 handler
fn build_app_handler() -> Handler {
    compose_middleware(
        api_handler(),
        &[log_middleware, auth_middleware, count_steps_middleware],
    )
}

fn run_tests() {
    let ok_req = Request {
        path: "/api/items".into(),
        auth_token: Some("secret".into()),
    };
    let bad_req = Request {
        path: "/api/items".into(),
        auth_token: None,
    };

    assert_eq!(handle_naive(bad_req.clone()).status, 401);
    assert_eq!(handle_naive(ok_req.clone()).status, 200);

    let app = build_app_handler();
    assert_eq!(app(bad_req.clone()).status, 401);
    assert_eq!(app(ok_req.clone()).status, 200);
    assert!(app(ok_req).body.contains("/api/items"));

    STEP_COUNTER.with(|c| *c.borrow_mut() = 0);
    let _ = app(Request {
        path: "/x".into(),
        auth_token: Some("secret".into()),
    });
    let steps = STEP_COUNTER.with(|c| *c.borrow());
    assert_eq!(steps, 2, "count middleware 应前后各 +1");

    // 单层 compose 等价于直接包一层
    let once = compose_middleware(api_handler(), &[auth_middleware]);
    assert_eq!(
        once(Request {
            path: "/".into(),
            auth_token: Some("secret".into()),
        })
        .status,
        200
    );

    println!("✅ Tower 中间件心智（迷你 Axum 对照）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ Tower 中间件心智 ************\n");
    println!("对照 Axum：`.layer(TraceLayer::new_for_http())` —— 本课用高阶函数叠洋葱。\n");

    let app = build_app_handler();
    for (path, token) in [("/health", Some("secret")), ("/health", None)] {
        let resp = app(Request {
            path: path.into(),
            auth_token: token.map(String::from),
        });
        println!("{path} token={token:?} -> {} {}", resp.status, resp.body);
    }

    run_tests();
}

fn main() {
    bootstrap();
}