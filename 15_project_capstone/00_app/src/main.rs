/*
[主题]: 毕业综合 —— 迷你 API（鉴权 + User/Item CRUD + Cache-Aside）
[难度]: Medium → Hard
[前置]: `08_web_axum`（Router / 中间件 / 分层）、`09_persistence`（Repository）、`10_caching_redis`（Cache-Aside）

一、主题说明
- 把就业主线串成**一块可放进作品集的心智模型**：不 bind 真端口、不连 PostgreSQL / Redis，全程 **std + 内存假实现**，verify 秒过。
- **鉴权**：对照 Axum 里 `Authorization: Bearer …` + 自定义 `FromRequestParts` 或 Tower `auth` 中间件。
- **CRUD**：`UserRepository` / `ItemRepository` trait + `InMemory*` —— 真项目把 impl 换成 **sqlx** `Pool` 即可。
- **缓存**：读 Item 走 Cache-Aside（miss 回源 Repo、回填 cache；写/删后 invalidate）。
- **路由**：`MiniRouter` 按 method + path 分发，对照 `axum::Router::route`。

二、写法对比
1. 直观版: `NaiveCapstone::dispatch` 一个巨大 `match`，鉴权、HashMap、缓存逻辑全揉在一起。
2. 进阶版: Auth → Router → Handler → Service → Repo / Cache 分层，与真实 Axum + sqlx + Redis 项目同构。

三、真实用途与生产对照
| 本课 | 生产 Rust 后端 |
| --- | --- |
| `AuthService::verify_bearer` | JWT / API Key + `tower_http::auth` 或自定义 Layer |
| `UserRepository` + `InMemoryUserRepo` | sqlx `query_as!` + `PgUserRepo` |
| `ItemService::get_cached` | Redis `GET` → miss → sqlx → `SETEX` |
| `MiniRouter::handle` | `axum::Router` + `MethodRouter` |
| `bootstrap` 打印 trace 行 | `tracing::info!` span（见 `14_observability`） |

资料: https://docs.rs/axum/latest/axum/
      https://docs.rs/sqlx/latest/sqlx/
      https://docs.rs/tracing/latest/tracing/
      https://www.amazon.com/Programming-Rust-Fast-Systems-Development/dp/1492052590
      https://www.manning.com/books/rust-servers-services-and-apps
      https://nostarch.com/rust-rustaceans
*/

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

// ══════════════════════════════════════════════════════════════════
// 迷你 HTTP（不 listen，只测逻辑）
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Clone, Debug)]
struct HttpRequest {
    method: HttpMethod,
    path: String,
    authorization: Option<String>,
    body: String,
}

#[derive(Clone, Debug, PartialEq)]
struct HttpResponse {
    status: u16,
    body: String,
}

impl HttpResponse {
    fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            body: body.into(),
        }
    }

    fn created(body: impl Into<String>) -> Self {
        Self {
            status: 201,
            body: body.into(),
        }
    }

    fn err(status: u16, msg: impl Into<String>) -> Self {
        Self {
            status,
            body: msg.into(),
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// 领域模型与统一错误
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq)]
struct User {
    id: u64,
    email: String,
}

#[derive(Clone, Debug, PartialEq)]
struct Item {
    id: u64,
    owner_id: u64,
    title: String,
}

#[derive(Debug, PartialEq)]
enum AppError {
    Unauthorized,
    NotFound(&'static str, u64),
    Conflict(&'static str),
    BadRequest(&'static str),
}

impl AppError {
    fn http_status(&self) -> u16 {
        match self {
            AppError::Unauthorized => 401,
            AppError::NotFound(_, _) => 404,
            AppError::Conflict(_) => 409,
            AppError::BadRequest(_) => 400,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Unauthorized => write!(f, "未授权"),
            AppError::NotFound(kind, id) => write!(f, "{kind} {id} 不存在"),
            AppError::Conflict(msg) => write!(f, "{msg}"),
            AppError::BadRequest(msg) => write!(f, "{msg}"),
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// 鉴权（对照 Bearer Token 校验）
// ══════════════════════════════════════════════════════════════════

/// 生产里可换成 JWT Claims / session id
const VALID_API_TOKEN: &str = "capstone-secret";

struct AuthService;

impl AuthService {
    /// 从 `Authorization: Bearer <token>` 提取并校验
    fn verify_bearer(header: Option<&str>) -> Result<(), AppError> {
        let Some(raw) = header else {
            return Err(AppError::Unauthorized);
        };
        let token = raw
            .strip_prefix("Bearer ")
            .or_else(|| raw.strip_prefix("bearer "))
            .ok_or(AppError::Unauthorized)?;
        if token.trim() == VALID_API_TOKEN {
            Ok(())
        } else {
            Err(AppError::Unauthorized)
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// Repository trait + InMemory（对照 sqlx）
// ══════════════════════════════════════════════════════════════════

trait UserRepository {
    fn find_by_id(&self, id: u64) -> Result<Option<User>, AppError>;
    fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    fn insert(&mut self, user: User) -> Result<(), AppError>;
    fn delete(&mut self, id: u64) -> Result<bool, AppError>;
}

trait ItemRepository {
    fn find_by_id(&self, id: u64) -> Result<Option<Item>, AppError>;
    fn insert(&mut self, item: Item) -> Result<(), AppError>;
    fn update(&mut self, item: Item) -> Result<(), AppError>;
    fn delete(&mut self, id: u64) -> Result<bool, AppError>;
}

struct InMemoryUserRepo {
    by_id: HashMap<u64, User>,
    by_email: HashMap<String, u64>,
}

impl InMemoryUserRepo {
    fn new() -> Self {
        Self {
            by_id: HashMap::new(),
            by_email: HashMap::new(),
        }
    }
}

impl UserRepository for InMemoryUserRepo {
    fn find_by_id(&self, id: u64) -> Result<Option<User>, AppError> {
        Ok(self.by_id.get(&id).cloned())
    }

    fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let key = email.to_lowercase();
        match self.by_email.get(&key) {
            Some(id) => self.find_by_id(*id),
            None => Ok(None),
        }
    }

    fn insert(&mut self, user: User) -> Result<(), AppError> {
        let key = user.email.to_lowercase();
        if self.by_email.contains_key(&key) {
            return Err(AppError::Conflict("邮箱已注册"));
        }
        self.by_email.insert(key, user.id);
        self.by_id.insert(user.id, user);
        Ok(())
    }

    fn delete(&mut self, id: u64) -> Result<bool, AppError> {
        match self.by_id.remove(&id) {
            Some(u) => {
                self.by_email.remove(&u.email.to_lowercase());
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

struct InMemoryItemRepo {
    rows: HashMap<u64, Item>,
}

impl InMemoryItemRepo {
    fn new() -> Self {
        Self {
            rows: HashMap::new(),
        }
    }
}

impl ItemRepository for InMemoryItemRepo {
    fn find_by_id(&self, id: u64) -> Result<Option<Item>, AppError> {
        Ok(self.rows.get(&id).cloned())
    }

    fn insert(&mut self, item: Item) -> Result<(), AppError> {
        if self.rows.contains_key(&item.id) {
            return Err(AppError::Conflict("Item id 冲突"));
        }
        self.rows.insert(item.id, item);
        Ok(())
    }

    fn update(&mut self, item: Item) -> Result<(), AppError> {
        if !self.rows.contains_key(&item.id) {
            return Err(AppError::NotFound("item", item.id));
        }
        self.rows.insert(item.id, item);
        Ok(())
    }

    fn delete(&mut self, id: u64) -> Result<bool, AppError> {
        Ok(self.rows.remove(&id).is_some())
    }
}

// ══════════════════════════════════════════════════════════════════
// Cache-Aside（仅 Item 读路径，对照 Redis）
// ══════════════════════════════════════════════════════════════════

#[derive(Clone)]
struct CacheEntry<T: Clone> {
    value: T,
    expires_at: Instant,
}

impl<T: Clone> CacheEntry<T> {
    fn fresh(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
        }
    }

    fn alive(&self) -> bool {
        Instant::now() < self.expires_at
    }
}

struct ItemCache {
    ttl: Duration,
    store: HashMap<String, CacheEntry<Item>>,
    pub hits: u32,
    pub misses: u32,
    pub invalidations: u32,
}

impl ItemCache {
    fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            store: HashMap::new(),
            hits: 0,
            misses: 0,
            invalidations: 0,
        }
    }

    fn key(id: u64) -> String {
        format!("item:{id}")
    }

    fn get(&mut self, id: u64) -> Option<Item> {
        let k = Self::key(id);
        match self.store.get(&k) {
            Some(entry) if entry.alive() => {
                self.hits += 1;
                Some(entry.value.clone())
            }
            Some(_) => {
                self.store.remove(&k);
                self.misses += 1;
                None
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    fn set(&mut self, item: &Item) {
        let k = Self::key(item.id);
        self.store
            .insert(k, CacheEntry::fresh(item.clone(), self.ttl));
    }

    fn invalidate(&mut self, id: u64) {
        if self.store.remove(&Self::key(id)).is_some() {
            self.invalidations += 1;
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// Service 层
// ══════════════════════════════════════════════════════════════════

struct UserService<'a, R: UserRepository + ?Sized> {
    repo: &'a mut R,
    next_id: u64,
}

impl<'a, R: UserRepository + ?Sized> UserService<'a, R> {
    fn create(&mut self, email: &str) -> Result<User, AppError> {
        let email = email.trim();
        if email.is_empty() || !email.contains('@') {
            return Err(AppError::BadRequest("邮箱格式无效"));
        }
        if self.repo.find_by_email(email)?.is_some() {
            return Err(AppError::Conflict("邮箱已注册"));
        }
        let user = User {
            id: self.next_id,
            email: email.to_string(),
        };
        self.next_id += 1;
        self.repo.insert(user.clone())?;
        Ok(user)
    }

    fn get(&self, id: u64) -> Result<User, AppError> {
        self.repo
            .find_by_id(id)?
            .ok_or(AppError::NotFound("user", id))
    }

    fn remove(&mut self, id: u64) -> Result<(), AppError> {
        if !self.repo.delete(id)? {
            return Err(AppError::NotFound("user", id));
        }
        Ok(())
    }
}

struct ItemService<'a, I: ItemRepository + ?Sized> {
    repo: &'a mut I,
    cache: &'a mut ItemCache,
    next_id: u64,
}

impl<'a, I: ItemRepository + ?Sized> ItemService<'a, I> {
    /// Cache-Aside 读路径
    fn get_cached(&mut self, id: u64) -> Result<Item, AppError> {
        if let Some(item) = self.cache.get(id) {
            return Ok(item);
        }
        match self.repo.find_by_id(id)? {
            Some(item) => {
                self.cache.set(&item);
                Ok(item)
            }
            None => Err(AppError::NotFound("item", id)),
        }
    }

    fn create(&mut self, owner_id: u64, title: &str) -> Result<Item, AppError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::BadRequest("标题不能为空"));
        }
        let item = Item {
            id: self.next_id,
            owner_id,
            title: title.to_string(),
        };
        self.next_id += 1;
        self.repo.insert(item.clone())?;
        Ok(item)
    }

    fn update_title(&mut self, id: u64, title: &str) -> Result<Item, AppError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::BadRequest("标题不能为空"));
        }
        let mut item = self
            .repo
            .find_by_id(id)?
            .ok_or(AppError::NotFound("item", id))?;
        item.title = title.to_string();
        self.repo.update(item.clone())?;
        self.cache.invalidate(id);
        Ok(item)
    }

    fn delete(&mut self, id: u64) -> Result<(), AppError> {
        if !self.repo.delete(id)? {
            return Err(AppError::NotFound("item", id));
        }
        self.cache.invalidate(id);
        Ok(())
    }
}

// ── 解析极简 body：`email=...` / `title=...` / `owner_id=...` ───

fn parse_field(body: &str, key: &str) -> Option<String> {
    body.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k.trim() == key {
            Some(v.trim().to_string())
        } else {
            None
        }
    })
}

fn parse_path_id(path: &str, prefix: &str) -> Option<u64> {
    path.strip_prefix(prefix)?.parse().ok()
}

// ══════════════════════════════════════════════════════════════════
// 直观版：全揉在一个结构里
// ══════════════════════════════════════════════════════════════════

struct NaiveCapstone {
    users: HashMap<u64, User>,
    items: HashMap<u64, Item>,
    item_cache: HashMap<u64, Item>,
    next_user: u64,
}

impl NaiveCapstone {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            items: HashMap::new(),
            item_cache: HashMap::new(),
            next_user: 1,
        }
    }

    fn dispatch(&mut self, req: HttpRequest) -> HttpResponse {
        if req.path == "/health" {
            return HttpResponse::ok("ok");
        }
        if AuthService::verify_bearer(req.authorization.as_deref()).is_err() {
            return HttpResponse::err(401, "unauthorized");
        }
        match (req.method, req.path.as_str()) {
            (HttpMethod::Post, "/users") => {
                let Some(email) = parse_field(&req.body, "email") else {
                    return HttpResponse::err(400, "missing email");
                };
                if email.is_empty() {
                    return HttpResponse::err(400, "bad email");
                }
                let id = self.next_user;
                self.next_user += 1;
                let user = User { id, email: email.clone() };
                self.users.insert(id, user.clone());
                HttpResponse::created(format!("user:{id}:{email}"))
            }
            (HttpMethod::Get, path) if path.starts_with("/users/") => {
                let Some(id) = parse_path_id(path, "/users/") else {
                    return HttpResponse::err(404, "not found");
                };
                match self.users.get(&id) {
                    Some(u) => HttpResponse::ok(format!("user:{}", u.email)),
                    None => HttpResponse::err(404, "user missing"),
                }
            }
            (HttpMethod::Get, path) if path.starts_with("/items/") => {
                let Some(id) = parse_path_id(path, "/items/") else {
                    return HttpResponse::err(404, "not found");
                };
                if let Some(item) = self.item_cache.get(&id) {
                    return HttpResponse::ok(format!("cache:{}", item.title));
                }
                match self.items.get(&id) {
                    Some(item) => {
                        self.item_cache.insert(id, item.clone());
                        HttpResponse::ok(format!("db:{}", item.title))
                    }
                    None => HttpResponse::err(404, "item missing"),
                }
            }
            _ => HttpResponse::err(404, "route not found"),
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// 进阶版：Router 串联 Handler（持有多层 Service）
// ══════════════════════════════════════════════════════════════════

struct AppState {
    users: InMemoryUserRepo,
    items: InMemoryItemRepo,
    cache: ItemCache,
    user_seq: u64,
    item_seq: u64,
}

impl AppState {
    fn new() -> Self {
        Self {
            users: InMemoryUserRepo::new(),
            items: InMemoryItemRepo::new(),
            cache: ItemCache::new(Duration::from_secs(120)),
            user_seq: 1,
            item_seq: 1,
        }
    }
}

struct MiniRouter {
    state: AppState,
}

impl MiniRouter {
    fn new() -> Self {
        Self {
            state: AppState::new(),
        }
    }

    /// 对照 `axum::Router::call`：统一入口
    fn handle(&mut self, req: HttpRequest) -> HttpResponse {
        if req.path == "/health" {
            return HttpResponse::ok("{\"status\":\"up\"}");
        }

        let needs_auth = !req.path.starts_with("/health");
        if needs_auth {
            if let Err(e) = AuthService::verify_bearer(req.authorization.as_deref()) {
                return HttpResponse::err(e.http_status(), e.to_string());
            }
        }

        match (req.method, req.path.as_str()) {
            (HttpMethod::Post, "/users") => self.post_user(&req.body),
            (HttpMethod::Get, path) if path.starts_with("/users/") => {
                self.get_user(path)
            }
            (HttpMethod::Delete, path) if path.starts_with("/users/") => {
                self.delete_user(path)
            }
            (HttpMethod::Post, "/items") => self.post_item(&req.body),
            (HttpMethod::Get, path) if path.starts_with("/items/") => self.get_item(path),
            (HttpMethod::Put, path) if path.starts_with("/items/") => {
                self.put_item(path, &req.body)
            }
            (HttpMethod::Delete, path) if path.starts_with("/items/") => {
                self.delete_item(path)
            }
            _ => HttpResponse::err(404, "route not found"),
        }
    }

    fn post_user(&mut self, body: &str) -> HttpResponse {
        let Some(email) = parse_field(body, "email") else {
            return HttpResponse::err(400, "missing email");
        };
        let mut svc = UserService {
            repo: &mut self.state.users,
            next_id: self.state.user_seq,
        };
        match svc.create(&email) {
            Ok(u) => {
                self.state.user_seq = svc.next_id;
                HttpResponse::created(format!(
                    "{{\"id\":{},\"email\":\"{}\"}}",
                    u.id, u.email
                ))
            }
            Err(e) => HttpResponse::err(e.http_status(), e.to_string()),
        }
    }

    fn get_user(&mut self, path: &str) -> HttpResponse {
        let Some(id) = parse_path_id(path, "/users/") else {
            return HttpResponse::err(404, "bad path");
        };
        let svc = UserService {
            repo: &mut self.state.users,
            next_id: self.state.user_seq,
        };
        match svc.get(id) {
            Ok(u) => HttpResponse::ok(format!(
                "{{\"id\":{},\"email\":\"{}\"}}",
                u.id, u.email
            )),
            Err(e) => HttpResponse::err(e.http_status(), e.to_string()),
        }
    }

    fn delete_user(&mut self, path: &str) -> HttpResponse {
        let Some(id) = parse_path_id(path, "/users/") else {
            return HttpResponse::err(404, "bad path");
        };
        let mut svc = UserService {
            repo: &mut self.state.users,
            next_id: self.state.user_seq,
        };
        match svc.remove(id) {
            Ok(()) => HttpResponse::ok("deleted"),
            Err(e) => HttpResponse::err(e.http_status(), e.to_string()),
        }
    }

    fn post_item(&mut self, body: &str) -> HttpResponse {
        let owner = parse_field(body, "owner_id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let Some(title) = parse_field(body, "title") else {
            return HttpResponse::err(400, "missing title");
        };
        let mut svc = ItemService {
            repo: &mut self.state.items,
            cache: &mut self.state.cache,
            next_id: self.state.item_seq,
        };
        match svc.create(owner, &title) {
            Ok(item) => {
                self.state.item_seq = svc.next_id;
                HttpResponse::created(format!(
                    "{{\"id\":{},\"title\":\"{}\"}}",
                    item.id, item.title
                ))
            }
            Err(e) => HttpResponse::err(e.http_status(), e.to_string()),
        }
    }

    fn get_item(&mut self, path: &str) -> HttpResponse {
        let Some(id) = parse_path_id(path, "/items/") else {
            return HttpResponse::err(404, "bad path");
        };
        let mut svc = ItemService {
            repo: &mut self.state.items,
            cache: &mut self.state.cache,
            next_id: self.state.item_seq,
        };
        match svc.get_cached(id) {
            Ok(item) => HttpResponse::ok(format!(
                "{{\"id\":{},\"title\":\"{}\",\"via_cache_stats\":\"hits={}\"}}",
                item.id, item.title, self.state.cache.hits
            )),
            Err(e) => HttpResponse::err(e.http_status(), e.to_string()),
        }
    }

    fn put_item(&mut self, path: &str, body: &str) -> HttpResponse {
        let Some(id) = parse_path_id(path, "/items/") else {
            return HttpResponse::err(404, "bad path");
        };
        let Some(title) = parse_field(body, "title") else {
            return HttpResponse::err(400, "missing title");
        };
        let mut svc = ItemService {
            repo: &mut self.state.items,
            cache: &mut self.state.cache,
            next_id: self.state.item_seq,
        };
        match svc.update_title(id, &title) {
            Ok(item) => HttpResponse::ok(format!(
                "{{\"id\":{},\"title\":\"{}\"}}",
                item.id, item.title
            )),
            Err(e) => HttpResponse::err(e.http_status(), e.to_string()),
        }
    }

    fn delete_item(&mut self, path: &str) -> HttpResponse {
        let Some(id) = parse_path_id(path, "/items/") else {
            return HttpResponse::err(404, "bad path");
        };
        let mut svc = ItemService {
            repo: &mut self.state.items,
            cache: &mut self.state.cache,
            next_id: self.state.item_seq,
        };
        match svc.delete(id) {
            Ok(()) => HttpResponse::ok("deleted"),
            Err(e) => HttpResponse::err(e.http_status(), e.to_string()),
        }
    }
}

fn auth_header_ok() -> Option<String> {
    Some(format!("Bearer {VALID_API_TOKEN}"))
}

fn run_tests() {
    // ── 鉴权：成功 / 失败 ───────────────────────────────────────
    assert!(AuthService::verify_bearer(Some("Bearer capstone-secret")).is_ok());
    assert_eq!(
        AuthService::verify_bearer(Some("Bearer wrong")),
        Err(AppError::Unauthorized)
    );
    assert_eq!(
        AuthService::verify_bearer(None),
        Err(AppError::Unauthorized)
    );

    // ── 直观版 ──────────────────────────────────────────────────
    let mut naive = NaiveCapstone::new();
    assert_eq!(
        naive
            .dispatch(HttpRequest {
                method: HttpMethod::Get,
                path: "/health".into(),
                authorization: None,
                body: String::new(),
            })
            .status,
        200
    );
    assert_eq!(
        naive
            .dispatch(HttpRequest {
                method: HttpMethod::Post,
                path: "/users".into(),
                authorization: None,
                body: "email=a@b.com".into(),
            })
            .status,
        401
    );
    let created = naive.dispatch(HttpRequest {
        method: HttpMethod::Post,
        path: "/users".into(),
        authorization: auth_header_ok(),
        body: "email=naive@test.io".into(),
    });
    assert_eq!(created.status, 201);

    // 直观版 Cache-Aside：第一次 db，第二次 cache
    let _ = naive.dispatch(HttpRequest {
        method: HttpMethod::Post,
        path: "/items".into(),
        authorization: auth_header_ok(),
        body: "title=book".into(),
    });
    // 直观版未实现 POST /items，走 404 —— 手动塞 item 测读缓存
    naive.items.insert(
        1,
        Item {
            id: 1,
            owner_id: 1,
            title: "Rust 书".into(),
        },
    );
    let first = naive.dispatch(HttpRequest {
        method: HttpMethod::Get,
        path: "/items/1".into(),
        authorization: auth_header_ok(),
        body: String::new(),
    });
    assert!(first.body.starts_with("db:"));
    let second = naive.dispatch(HttpRequest {
        method: HttpMethod::Get,
        path: "/items/1".into(),
        authorization: auth_header_ok(),
        body: String::new(),
    });
    assert!(second.body.starts_with("cache:"));

    // ── 进阶 Router：鉴权失败 ───────────────────────────────────
    let mut router = MiniRouter::new();
    assert_eq!(
        router
            .handle(HttpRequest {
                method: HttpMethod::Get,
                path: "/users/1".into(),
                authorization: None,
                body: String::new(),
            })
            .status,
        401
    );

    // User CRUD 成功
    let u_resp = router.handle(HttpRequest {
        method: HttpMethod::Post,
        path: "/users".into(),
        authorization: auth_header_ok(),
        body: "email=alice@capstone.dev".into(),
    });
    assert_eq!(u_resp.status, 201);
    assert!(u_resp.body.contains("alice@capstone.dev"));

    let uid: u64 = u_resp
        .body
        .split("\"id\":")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.parse().ok())
        .expect("parse user id");

    let u_get = router.handle(HttpRequest {
        method: HttpMethod::Get,
        path: format!("/users/{uid}"),
        authorization: auth_header_ok(),
        body: String::new(),
    });
    assert_eq!(u_get.status, 200);

    // User 冲突 / 404
    let dup = router.handle(HttpRequest {
        method: HttpMethod::Post,
        path: "/users".into(),
        authorization: auth_header_ok(),
        body: "email=alice@capstone.dev".into(),
    });
    assert_eq!(dup.status, 409);

    assert_eq!(
        router
            .handle(HttpRequest {
                method: HttpMethod::Get,
                path: "/users/9999".into(),
                authorization: auth_header_ok(),
                body: String::new(),
            })
            .status,
        404
    );

    let bad_email = router.handle(HttpRequest {
        method: HttpMethod::Post,
        path: "/users".into(),
        authorization: auth_header_ok(),
        body: "email=not-an-email".into(),
    });
    assert_eq!(bad_email.status, 400);

    // Item CRUD + Cache-Aside
    let i_resp = router.handle(HttpRequest {
        method: HttpMethod::Post,
        path: "/items".into(),
        authorization: auth_header_ok(),
        body: format!("owner_id={uid}&title=Mini API"),
    });
    assert_eq!(i_resp.status, 201);
    let iid: u64 = i_resp
        .body
        .split("\"id\":")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.parse().ok())
        .expect("parse item id");

    let miss = router.handle(HttpRequest {
        method: HttpMethod::Get,
        path: format!("/items/{iid}"),
        authorization: auth_header_ok(),
        body: String::new(),
    });
    assert_eq!(miss.status, 200);
    let hits_after_first = router.state.cache.hits;
    assert_eq!(router.state.cache.misses, 1);

    let hit = router.handle(HttpRequest {
        method: HttpMethod::Get,
        path: format!("/items/{iid}"),
        authorization: auth_header_ok(),
        body: String::new(),
    });
    assert_eq!(hit.status, 200);
    assert!(router.state.cache.hits > hits_after_first);

    let updated = router.handle(HttpRequest {
        method: HttpMethod::Put,
        path: format!("/items/{iid}"),
        authorization: auth_header_ok(),
        body: "title=Mini API v2".into(),
    });
    assert_eq!(updated.status, 200);
    assert!(updated.body.contains("v2"));
    assert!(router.state.cache.invalidations >= 1);

    // 更新后再次读应 miss 后回源（invalidate 后 cache 空）
    let before_misses = router.state.cache.misses;
    let after_update_get = router.handle(HttpRequest {
        method: HttpMethod::Get,
        path: format!("/items/{iid}"),
        authorization: auth_header_ok(),
        body: String::new(),
    });
    assert_eq!(after_update_get.status, 200);
    assert!(router.state.cache.misses > before_misses);

    let del = router.handle(HttpRequest {
        method: HttpMethod::Delete,
        path: format!("/items/{iid}"),
        authorization: auth_header_ok(),
        body: String::new(),
    });
    assert_eq!(del.status, 200);
    assert_eq!(
        router
            .handle(HttpRequest {
                method: HttpMethod::Get,
                path: format!("/items/{iid}"),
                authorization: auth_header_ok(),
                body: String::new(),
            })
            .status,
        404
    );

    let del_user = router.handle(HttpRequest {
        method: HttpMethod::Delete,
        path: format!("/users/{uid}"),
        authorization: auth_header_ok(),
        body: String::new(),
    });
    assert_eq!(del_user.status, 200);

    println!("✅ 毕业综合迷你 API（鉴权+CRUD+Cache-Aside）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 毕业综合 · l00_mini_api_capstone ************\n");
    println!("对照生产：Axum Router + sqlx Repo + Redis Cache-Aside + tracing");
    println!("本课纯内存模拟，不 bind 端口、不连 DB/Redis。\n");

    let mut router = MiniRouter::new();

    let health = router.handle(HttpRequest {
        method: HttpMethod::Get,
        path: "/health".into(),
        authorization: None,
        body: String::new(),
    });
    println!("[trace] GET /health -> {} {}", health.status, health.body);

    let user = router.handle(HttpRequest {
        method: HttpMethod::Post,
        path: "/users".into(),
        authorization: auth_header_ok(),
        body: "email=demo@krs.local".into(),
    });
    println!("[trace] POST /users -> {} {}", user.status, user.body);

    let item = router.handle(HttpRequest {
        method: HttpMethod::Post,
        path: "/items".into(),
        authorization: auth_header_ok(),
        body: "owner_id=1&title=demo-item".into(),
    });
    println!("[trace] POST /items -> {} {}", item.status, item.body);

    if let Some(id) = item
        .body
        .split("\"id\":")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<u64>().ok())
    {
        for n in 1..=2 {
            let r = router.handle(HttpRequest {
                method: HttpMethod::Get,
                path: format!("/items/{id}"),
                authorization: auth_header_ok(),
                body: String::new(),
            });
            println!(
                "[trace] GET /items/{id} 第{n}次 -> {} (cache hits={})",
                r.status, router.state.cache.hits
            );
        }
    }

    println!("\n下一步：把 `InMemory*Repo` 换成 sqlx，把 `ItemCache` 换成 redis crate。");
    run_tests();
}

fn main() {
    bootstrap();
}