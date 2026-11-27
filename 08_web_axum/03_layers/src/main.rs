/*
[主题]: Handler → Service → Repository 分层（Axum State + 业务心智力）
[难度]: Medium
[前置]: 本阶段前三课、`04_traits_generics`

一、主题说明
- 别把 SQL 写在 handler 里：典型三层 **HTTP Handler**（解析 HTTP）→ **Service**（业务规则）→ **Repo**（存取数据）。
- Axum 里：`State<AppState>` 注入 `UserService`；handler 只调 `service.create(...)`。
- 本课：trait 定义边界，内存 `HashMap` 假 Repo，**全程无网络**，verify 秒过。

二、写法对比
1. 直观版: handler 里直接改 `HashMap`。
2. 进阶版: `UserHandler` 持 `Arc<dyn UserService>`，Service 持 `Arc<dyn UserRepo>`。

三、真实用途与对照
- 与真实项目一致：后续 `09_persistence` 把 `UserRepo` 换成 sqlx 实现即可。
- Axum State：https://docs.rs/axum/latest/axum/extract/struct.State.html
资料: https://docs.rs/axum/latest/axum/
      https://github.com/tokio-rs/axum
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
struct User {
    id: u64,
    name: String,
}

#[derive(Clone, Debug)]
struct CreateUserInput {
    name: String,
}

#[derive(Debug, PartialEq)]
enum AppError {
    NotFound,
    Duplicate,
    InvalidName,
}

impl AppError {
    fn http_status(&self) -> u16 {
        match self {
            AppError::NotFound => 404,
            AppError::Duplicate => 409,
            AppError::InvalidName => 400,
        }
    }
}

/// 对照数据库访问层（sqlx::Pool 的另一侧是 trait）
trait UserRepo: Send + Sync {
    fn insert(&self, name: String) -> Result<User, AppError>;
    fn find_by_id(&self, id: u64) -> Result<User, AppError>;
    fn exists_name(&self, name: &str) -> bool;
}

/// 对照业务 Service
trait UserService: Send + Sync {
    fn register(&self, input: CreateUserInput) -> Result<User, AppError>;
    fn get(&self, id: u64) -> Result<User, AppError>;
}

#[derive(Clone, Default)]
struct InMemoryUserRepo {
    inner: Arc<Mutex<HashMap<u64, User>>>,
    next_id: Arc<Mutex<u64>>,
}

impl InMemoryUserRepo {
    fn new() -> Self {
        Self::default()
    }
}

impl UserRepo for InMemoryUserRepo {
    fn insert(&self, name: String) -> Result<User, AppError> {
        let mut id_lock = self.next_id.lock().expect("mutex");
        let id = *id_lock;
        *id_lock = id.saturating_add(1);
        let user = User { id, name: name.clone() };
        self.inner.lock().expect("mutex").insert(id, user.clone());
        Ok(user)
    }

    fn find_by_id(&self, id: u64) -> Result<User, AppError> {
        self.inner
            .lock()
            .expect("mutex")
            .get(&id)
            .cloned()
            .ok_or(AppError::NotFound)
    }

    fn exists_name(&self, name: &str) -> bool {
        self.inner
            .lock()
            .expect("mutex")
            .values()
            .any(|u| u.name == name)
    }
}

struct UserServiceImpl<R: UserRepo> {
    repo: Arc<R>,
}

impl<R: UserRepo> UserServiceImpl<R> {
    fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

impl<R: UserRepo + 'static> UserService for UserServiceImpl<R> {
    fn register(&self, input: CreateUserInput) -> Result<User, AppError> {
        let name = input.name.trim();
        if name.is_empty() || name.len() > 32 {
            return Err(AppError::InvalidName);
        }
        if self.repo.exists_name(name) {
            return Err(AppError::Duplicate);
        }
        self.repo.insert(name.to_string())
    }

    fn get(&self, id: u64) -> Result<User, AppError> {
        self.repo.find_by_id(id)
    }
}

/// 假 HTTP 响应
#[derive(Debug, PartialEq)]
struct HttpResponse {
    status: u16,
    body: String,
}

/// 对照 Axum handler + State<AppState>
struct UserHandler<S: UserService + ?Sized> {
    service: Arc<S>,
}

impl<S: UserService + ?Sized> UserHandler<S> {
    fn new(service: Arc<S>) -> Self {
        Self { service }
    }

    fn post_users(&self, json_body: &str) -> HttpResponse {
        let name = parse_name_naive(json_body);
        match self.service.register(CreateUserInput { name }) {
            Ok(u) => HttpResponse {
                status: 201,
                body: format!("{{\"id\":{},\"name\":\"{}\"}}", u.id, u.name),
            },
            Err(e) => HttpResponse {
                status: e.http_status(),
                body: format!("err:{:?}", e),
            },
        }
    }

    fn get_user(&self, id: u64) -> HttpResponse {
        match self.service.get(id) {
            Ok(u) => HttpResponse {
                status: 200,
                body: format!("{{\"id\":{},\"name\":\"{}\"}}", u.id, u.name),
            },
            Err(e) => HttpResponse {
                status: e.http_status(),
                body: format!("err:{:?}", e),
            },
        }
    }
}

fn parse_name_naive(json_body: &str) -> String {
    json_body
        .split('"')
        .nth(3)
        .unwrap_or("")
        .to_string()
}

/// 直观版：handler 直连 HashMap（难测、难换存储）
struct NaiveApp {
    db: Mutex<HashMap<u64, User>>,
    next_id: Mutex<u64>,
}

impl NaiveApp {
    fn post_user(&self, name: &str) -> Result<User, AppError> {
        if name.trim().is_empty() {
            return Err(AppError::InvalidName);
        }
        let mut id = self.next_id.lock().unwrap();
        let user = User {
            id: *id,
            name: name.to_string(),
        };
        *id += 1;
        self.db.lock().unwrap().insert(user.id, user.clone());
        Ok(user)
    }
}

fn run_tests() {
    let naive = NaiveApp {
        db: Mutex::new(HashMap::new()),
        next_id: Mutex::new(1),
    };
    let u = naive.post_user("Tom").unwrap();
    assert_eq!(u.name, "Tom");

    let repo = Arc::new(InMemoryUserRepo::new());
    let service: Arc<dyn UserService> = Arc::new(UserServiceImpl::new(repo.clone()));
    let handler = UserHandler::new(service);

    let created = handler.post_users(r#"{"name":"Alice"}"#);
    assert_eq!(created.status, 201);
    assert!(created.body.contains("Alice"));

    let dup = handler.post_users(r#"{"name":"Alice"}"#);
    assert_eq!(dup.status, 409);

    let id: u64 = created
        .body
        .split("id\":")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let got = handler.get_user(id);
    assert_eq!(got.status, 200);
    assert!(got.body.contains("Alice"));

    assert_eq!(handler.get_user(9999).status, 404);

    let bad = handler.post_users(r#"{"name":""}"#);
    assert_eq!(bad.status, 400);

    println!("✅ Handler→Service→Repo 分层（迷你 Axum 对照）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 分层架构 ************\n");
    println!("对照 Axum：State<AppState> 注入 Service；Repo 可换 sqlx 实现。\n");

    let repo = Arc::new(InMemoryUserRepo::new());
    let service: Arc<dyn UserService> = Arc::new(UserServiceImpl::new(repo));
    let handler = UserHandler::new(service);

    let resp = handler.post_users(r#"{"name":"Boot"}"#);
    println!("POST /users -> {} {}", resp.status, resp.body);

    run_tests();
}

fn main() {
    bootstrap();
}