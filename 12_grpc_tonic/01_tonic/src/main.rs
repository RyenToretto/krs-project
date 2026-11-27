/*
[主题]: tonic / gRPC 心智 —— 一元 RPC、Status、Stub/Server（纯内存，无 HTTP/2）
[难度]: Medium → Hard
[前置]: `12_grpc_tonic/00_split`（服务边界）、`07_async_tokio`（真 tonic 全异步，本课先同步铺垫）

一、主题说明
- **gRPC**：IDL（proto）定义服务；客户端 **Stub** 发请求，服务端 **Server** 实现 trait；载荷常为 **protobuf** 二进制。
- **tonic**：Rust 上常用的 gRPC 栈（基于 tokio + hyper + prost）；`tonic_build` 从 `.proto` 生成 `UserServiceClient` / `UserServiceServer`。
- 本课用 `GrpcUnaryCall` + `RpcStatus` 模拟 **一元调用**（unary）：一次请求 ↔ 一次响应；流式 RPC 在注释里对照 `tonic::Streaming`。
- gRPC 路径形如 `/package.ServiceName/MethodName`，与 REST 的「资源路径」不同。

二、写法对比
1. 直观版: `fn get_user(id) -> Option<User>` 直接函数调用，无状态码、无 metadata 概念。
2. 进阶版: `MiniGrpcServer::invoke(method, bytes)` 返回 `Result<Bytes, RpcStatus>`；`MiniStub::get_user` 打包请求路径与序列化。

三、真实用途与对照
- `RpcStatus::not_found` ↔ `tonic::Status::not_found("...")`
- `Server::register` ↔ `UserServiceServer::new(svc).add_service(...)`
- 真项目：`cargo add tonic prost` + build.rs 编译 proto；本课用 UTF-8 字符串假装 protobuf。
资料: https://docs.rs/tonic/latest/tonic/
      https://github.com/hyperium/tonic
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;
use std::fmt;

// ── 迷你 protobuf：真实场景用 prost 编解码 ────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
struct GetUserRequest {
    user_id: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GetUserResponse {
    user_id: u64,
    email: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CreateUserRequest {
    email: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CreateUserResponse {
    user_id: u64,
}

/// 假装「序列化」—— 对照 prost::Message::encode
fn encode_get_user_req(r: &GetUserRequest) -> String {
    format!("gid:{}", r.user_id)
}

fn decode_get_user_req(payload: &str) -> Option<GetUserRequest> {
    payload
        .strip_prefix("gid:")
        .and_then(|s| s.parse().ok())
        .map(|user_id| GetUserRequest { user_id })
}

fn encode_get_user_resp(r: &GetUserResponse) -> String {
    format!("gur:{}:{}", r.user_id, r.email)
}

fn decode_get_user_resp(payload: &str) -> Option<GetUserResponse> {
    let rest = payload.strip_prefix("gur:")?;
    let (id, email) = rest.split_once(':')?;
    Some(GetUserResponse {
        user_id: id.parse().ok()?,
        email: email.to_string(),
    })
}

fn encode_create_user_req(r: &CreateUserRequest) -> String {
    format!("cre:{}", r.email)
}

fn decode_create_user_req(payload: &str) -> Option<CreateUserRequest> {
    payload
        .strip_prefix("cre:")
        .map(|email| CreateUserRequest {
            email: email.to_string(),
        })
}

fn encode_create_user_resp(r: &CreateUserResponse) -> String {
    format!("crr:{}", r.user_id)
}

fn decode_create_user_resp(payload: &str) -> Option<CreateUserResponse> {
    payload
        .strip_prefix("crr:")
        .and_then(|s| s.parse().ok())
        .map(|user_id| CreateUserResponse { user_id })
}

// ── gRPC Status（对照 tonic::Code）────────────────────────────────

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GrpcCode {
    Ok,
    NotFound,
    AlreadyExists,
    InvalidArgument,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RpcStatus {
    code: GrpcCode,
    message: String,
}

#[allow(dead_code)]
impl RpcStatus {
    fn ok() -> Self {
        Self {
            code: GrpcCode::Ok,
            message: String::new(),
        }
    }

    fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: GrpcCode::NotFound,
            message: msg.into(),
        }
    }

    fn already_exists(msg: impl Into<String>) -> Self {
        Self {
            code: GrpcCode::AlreadyExists,
            message: msg.into(),
        }
    }

    fn invalid_argument(msg: impl Into<String>) -> Self {
        Self {
            code: GrpcCode::InvalidArgument,
            message: msg.into(),
        }
    }
}

impl fmt::Display for RpcStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

// ── 服务端实现（对照 #[tonic::async_trait] impl UserService for Svc）─

struct UserRecord {
    id: u64,
    email: String,
}

struct UserGrpcService {
    by_id: HashMap<u64, UserRecord>,
    by_email: HashMap<String, u64>,
    next_id: u64,
}

impl UserGrpcService {
    fn new() -> Self {
        Self {
            by_id: HashMap::new(),
            by_email: HashMap::new(),
            next_id: 1,
        }
    }

    fn get_user(&self, req: GetUserRequest) -> Result<GetUserResponse, RpcStatus> {
        match self.by_id.get(&req.user_id) {
            Some(u) => Ok(GetUserResponse {
                user_id: u.id,
                email: u.email.clone(),
            }),
            None => Err(RpcStatus::not_found(format!("user {}", req.user_id))),
        }
    }

    fn create_user(&mut self, req: CreateUserRequest) -> Result<CreateUserResponse, RpcStatus> {
        let key = req.email.to_lowercase();
        if req.email.is_empty() {
            return Err(RpcStatus::invalid_argument("email 不能为空"));
        }
        if self.by_email.contains_key(&key) {
            return Err(RpcStatus::already_exists(req.email));
        }
        let id = self.next_id;
        self.next_id += 1;
        self.by_email.insert(key, id);
        self.by_id.insert(
            id,
            UserRecord {
                id,
                email: req.email,
            },
        );
        Ok(CreateUserResponse { user_id: id })
    }
}

type RpcHandler = fn(&mut UserGrpcService, &str) -> Result<String, RpcStatus>;

struct MiniGrpcServer {
    service: UserGrpcService,
    /// method 全名 → 处理函数（tonic 编译期生成 match）
    methods: HashMap<String, RpcHandler>,
}

impl MiniGrpcServer {
    fn new() -> Self {
        let mut methods: HashMap<String, RpcHandler> = HashMap::new();
        methods.insert(
            "/krs.user.v1.UserService/GetUser".into(),
            |svc, payload| {
                let req = decode_get_user_req(payload)
                    .ok_or_else(|| RpcStatus::invalid_argument("无法解析 GetUserRequest"))?;
                let resp = svc.get_user(req)?;
                Ok(encode_get_user_resp(&resp))
            },
        );
        methods.insert(
            "/krs.user.v1.UserService/CreateUser".into(),
            |svc, payload| {
                let req = decode_create_user_req(payload)
                    .ok_or_else(|| RpcStatus::invalid_argument("无法解析 CreateUserRequest"))?;
                let resp = svc.create_user(req)?;
                Ok(encode_create_user_resp(&resp))
            },
        );
        Self {
            service: UserGrpcService::new(),
            methods,
        }
    }

    /// 模拟 tonic 收到 HTTP/2 帧后路由到具体 RPC
    fn invoke(&mut self, full_method: &str, payload: &str) -> Result<String, RpcStatus> {
        let handler = self
            .methods
            .get(full_method)
            .ok_or_else(|| RpcStatus {
                code: GrpcCode::Internal,
                message: format!("未知方法: {}", full_method),
            })?;
        handler(&mut self.service, payload)
    }
}

// ── 客户端 Stub（对照 UserServiceClient）──────────────────────────

struct MiniUserStub<'a> {
    server: &'a mut MiniGrpcServer,
}

impl<'a> MiniUserStub<'a> {
    fn get_user(&mut self, user_id: u64) -> Result<GetUserResponse, RpcStatus> {
        let req = GetUserRequest { user_id };
        let body = encode_get_user_req(&req);
        let out = self
            .server
            .invoke("/krs.user.v1.UserService/GetUser", &body)?;
        decode_get_user_resp(&out)
            .ok_or_else(|| RpcStatus::invalid_argument("响应解码失败"))
    }

    fn create_user(&mut self, email: impl Into<String>) -> Result<CreateUserResponse, RpcStatus> {
        let req = CreateUserRequest { email: email.into() };
        let body = encode_create_user_req(&req);
        let out = self
            .server
            .invoke("/krs.user.v1.UserService/CreateUser", &body)?;
        decode_create_user_resp(&out)
            .ok_or_else(|| RpcStatus::invalid_argument("响应解码失败"))
    }
}

// ── 直观版：无 Status、无 method path ──────────────────────────────

struct NaiveUserApi {
    users: HashMap<u64, String>,
}

impl NaiveUserApi {
    fn get(&self, id: u64) -> Option<String> {
        self.users.get(&id).cloned()
    }
}

fn run_tests() {
    let naive = NaiveUserApi {
        users: HashMap::from([(1, "n@a.com".into())]),
    };
    assert_eq!(naive.get(1), Some("n@a.com".into()));
    assert_eq!(naive.get(2), None);

    let mut server = MiniGrpcServer::new();
    let mut stub = MiniUserStub { server: &mut server };

    let created = stub.create_user("grpc@krs.dev").unwrap();
    assert!(created.user_id >= 1);

    let got = stub.get_user(created.user_id).unwrap();
    assert_eq!(got.email, "grpc@krs.dev");

    let missing = stub.get_user(40404);
    assert_eq!(missing, Err(RpcStatus::not_found("user 40404")));

    let dup = stub.create_user("grpc@krs.dev");
    assert_eq!(
        dup,
        Err(RpcStatus::already_exists("grpc@krs.dev".to_string()))
    );

    let empty = stub.create_user("");
    assert_eq!(
        empty,
        Err(RpcStatus::invalid_argument("email 不能为空".to_string()))
    );

    let unknown = stub.server.invoke("/fake/Method", "x");
    assert!(unknown.is_err());

    println!("✅ tonic/gRPC 心智（Unary + Status + Stub/Server）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 tonic/gRPC 心智 ************\n");

    let mut server = MiniGrpcServer::new();
    let mut stub = MiniUserStub { server: &mut server };

    match stub.create_user("learner@tonic.dev") {
        Ok(r) => println!("CreateUser → user_id={}", r.user_id),
        Err(s) => println!("CreateUser 失败: {}", s),
    }

    match stub.get_user(1) {
        Ok(r) => println!("GetUser → {:?}", r),
        Err(s) => println!("GetUser: {}", s),
    }

    println!(
        "\n流式 RPC（未实现）: 服务端返回 Iterator / tonic::Streaming<T>；客户端 while let Some(msg) = stream.next().await"
    );
    println!("真 tonic 示例仓库: https://github.com/hyperium/tonic\n");

    run_tests();
}

fn main() {
    bootstrap();
}