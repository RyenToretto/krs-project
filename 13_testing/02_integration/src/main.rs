/*
[主题]: 集成测试心智 —— 同进程多模块联调（模拟 tests/ 集成测）
[难度]: Medium
[前置]: `08_web_axum`（分层）、`13_testing/00_unit`、`13_testing/01_mock`

一、主题说明
- **单元测试**测单函数；**集成测试**验多条模块拼起来的行为（API → Service → Repo），常连真 DB 或至少真内存栈。
- 标准 Rust 布局：crate 根下 `tests/integration_test.rs`，`cargo test` 会把每个文件编成**独立 crate** 只访问库的 public API（见 Book ch11-02）。
- 本课在**同一 `main.rs` 进程**里划分 `api` / `service` / `repo` 模块并端到端断言，offline、verify 友好；注释标明如何搬到 `tests/`。

二、写法对比
1. 直观版: 一个大函数里注册、查库、返回 JSON 全写一起，「集成」只能手动跑 main。
2. 进阶版: 分层 + `AppHarness::new()` 一次性布线；`run_integration_cases()` 表驱动走完整 HTTP 风格请求。

三、真实用途
- CI 里 `cargo test --test integration_test`；本地用 testcontainers 起 Postgres 的版本见 `09_persistence`。
- 粗基准：对整条 `POST /users` 链路计时，回归用 Criterion。
资料: https://doc.rust-lang.org/book/ch11-00-testing.html
      https://doc.rust-lang.org/book/ch11-02-running-tests.html
      https://doc.rust-lang.org/rust-by-example/testing.html
*/

mod repo {
    use std::collections::HashMap;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct User {
        pub id: u64,
        pub email: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum RepoError {
        DuplicateEmail,
        NotFound(u64),
    }

    /// 仓储 —— 生产为 sqlx；本课内存实现。
    pub trait UserRepo {
        fn create(&mut self, email: &str) -> Result<User, RepoError>;
        fn get(&self, id: u64) -> Result<User, RepoError>;
    }

    pub struct InMemoryUserRepo {
        users: HashMap<u64, User>,
        emails: HashMap<String, u64>,
        next_id: u64,
    }

    impl InMemoryUserRepo {
        pub fn new() -> Self {
            Self {
                users: HashMap::new(),
                emails: HashMap::new(),
                next_id: 1,
            }
        }
    }

    impl UserRepo for InMemoryUserRepo {
        fn create(&mut self, email: &str) -> Result<User, RepoError> {
            let key = email.to_lowercase();
            if self.emails.contains_key(&key) {
                return Err(RepoError::DuplicateEmail);
            }
            let id = self.next_id;
            self.next_id += 1;
            let u = User {
                id,
                email: email.to_string(),
            };
            self.emails.insert(key, id);
            self.users.insert(id, u.clone());
            Ok(u)
        }

        fn get(&self, id: u64) -> Result<User, RepoError> {
            self.users
                .get(&id)
                .cloned()
                .ok_or(RepoError::NotFound(id))
        }
    }
}

mod service {
    use super::repo::{RepoError, User, UserRepo};

    pub struct UserService<R: UserRepo> {
        repo: R,
    }

    impl<R: UserRepo> UserService<R> {
        pub fn new(repo: R) -> Self {
            Self { repo }
        }

        pub fn register(&mut self, email: &str) -> Result<User, RepoError> {
            if !email.contains('@') {
                return Err(RepoError::NotFound(0)); // 简化：真实项目用独立 ValidationError
            }
            self.repo.create(email)
        }

        pub fn profile(&self, id: u64) -> Result<User, RepoError> {
            self.repo.get(id)
        }
    }
}

mod api {
    use super::service::UserService;
    use super::repo::{RepoError, UserRepo};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HttpStatus {
        Ok200,
        BadRequest400,
        Conflict409,
        NotFound404,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct HttpResponse {
        pub status: HttpStatus,
        pub body: String,
    }

    /// 迷你「路由」：类似 Axum Handler 调用 Service。
    pub struct UserApi<R: UserRepo> {
        service: UserService<R>,
    }

    impl<R: UserRepo> UserApi<R> {
        pub fn new(service: UserService<R>) -> Self {
            Self { service }
        }

        pub fn post_register(&mut self, email: &str) -> HttpResponse {
            match self.service.register(email) {
                Ok(u) => HttpResponse {
                    status: HttpStatus::Ok200,
                    body: format!("{{\"id\":{},\"email\":\"{}\"}}", u.id, u.email),
                },
                Err(RepoError::DuplicateEmail) => HttpResponse {
                    status: HttpStatus::Conflict409,
                    body: "{\"error\":\"duplicate_email\"}".into(),
                },
                Err(_) => HttpResponse {
                    status: HttpStatus::BadRequest400,
                    body: "{\"error\":\"bad_request\"}".into(),
                },
            }
        }

        pub fn get_profile(&self, id: u64) -> HttpResponse {
            match self.service.profile(id) {
                Ok(u) => HttpResponse {
                    status: HttpStatus::Ok200,
                    body: format!("{{\"id\":{},\"email\":\"{}\"}}", u.id, u.email),
                },
                Err(RepoError::NotFound(_)) => HttpResponse {
                    status: HttpStatus::NotFound404,
                    body: "{\"error\":\"not_found\"}".into(),
                },
                Err(RepoError::DuplicateEmail) => HttpResponse {
                    status: HttpStatus::BadRequest400,
                    body: "{}".into(),
                },
            }
        }
    }
}

use api::{HttpStatus, UserApi};
use repo::InMemoryUserRepo;
use service::UserService;
use std::time::Instant;

/// 进阶版：一条链路里完成 Repo → Service → Api 布线（等同集成测试 fixture）。
struct AppHarness {
    api: UserApi<InMemoryUserRepo>,
}

impl AppHarness {
    fn new() -> Self {
        let repo = InMemoryUserRepo::new();
        let service = UserService::new(repo);
        let api = UserApi::new(service);
        Self { api }
    }

    fn post_register(&mut self, email: &str) -> api::HttpResponse {
        self.api.post_register(email)
    }

    fn get_profile(&self, id: u64) -> api::HttpResponse {
        self.api.get_profile(id)
    }
}

/// 直观版：不分层，难以单独测某一层。
fn naive_register_flow(email: &str) -> (u16, String) {
    if !email.contains('@') {
        return (400, "bad".into());
    }
    (200, format!("ok:{}", email))
}

#[derive(Debug, Clone, Copy)]
struct IntegrationCase {
    name: &'static str,
    email: &'static str,
    want_status: HttpStatus,
    profile_after: bool,
}

const FLOW_CASES: &[IntegrationCase] = &[
    IntegrationCase {
        name: "注册成功并可查询",
        email: "alice@krs.dev",
        want_status: HttpStatus::Ok200,
        profile_after: true,
    },
    IntegrationCase {
        name: "重复邮箱冲突",
        email: "alice@krs.dev",
        want_status: HttpStatus::Conflict409,
        profile_after: false,
    },
    IntegrationCase {
        name: "非法邮箱",
        email: "not-email",
        want_status: HttpStatus::BadRequest400,
        profile_after: false,
    },
];

fn run_integration_table() {
    let mut app = AppHarness::new();
    let mut last_id = 0_u64;
    for case in FLOW_CASES {
        let resp = app.post_register(case.email);
        assert_eq!(
            resp.status, case.want_status,
            "用例「{}」状态码",
            case.name
        );
        if case.profile_after {
            let id: u64 = resp
                .body
                .split("\"id\":")
                .nth(1)
                .and_then(|s| s.split(',').next())
                .and_then(|s| s.trim().parse().ok())
                .expect("解析 id");
            last_id = id;
            let get = app.get_profile(id);
            assert_eq!(get.status, HttpStatus::Ok200);
            assert!(get.body.contains(case.email));
        }
        let _ = last_id;
    }
}

fn bench_register_flow(n: u32) -> std::time::Duration {
    let start = Instant::now();
    for i in 0..n {
        let mut app = AppHarness::new();
        let email = format!("user{}@bench.krs", i);
        let _ = app.post_register(&email);
    }
    start.elapsed()
}

fn run_tests() {
    assert_eq!(naive_register_flow("a@b.c").0, 200);
    assert_eq!(naive_register_flow("nope").0, 400);

    let mut app = AppHarness::new();
    let r1 = app.post_register("solo@krs.dev");
    assert_eq!(r1.status, HttpStatus::Ok200);
    let r2 = app.post_register("solo@krs.dev");
    assert_eq!(r2.status, HttpStatus::Conflict409);

    run_integration_table();

    let dur = bench_register_flow(2_000);
    assert!(dur.as_secs() < 10, "联调粗基准超时");

    println!("✅ 集成测试心智（同进程多模块联调）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 集成测试心智 ************\n");

    let (code, body) = naive_register_flow("naive@demo.dev");
    println!("直观版单函数: HTTP {} → {}", code, body);

    let mut app = AppHarness::new();
    let reg = app.post_register("demo@krs.dev");
    println!("进阶版 POST /users: {:?} {}", reg.status, reg.body);
    let id: u64 = reg
        .body
        .split("\"id\":")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(1);
    let prof = app.get_profile(id);
    println!("GET /users/{}: {:?} {}", id, prof.status, prof.body);

    println!("\n搬到真集成测（注释，本课不建 tests/ 目录）:");
    println!("  1. 把 repo/service/api 抽到 lib.rs 并 pub use;");
    println!("  2. 新建 tests/integration_test.rs，只调用公开 API;");
    println!("  3. cargo test --test integration_test");
    println!("  文档: https://doc.rust-lang.org/book/ch11-02-running-tests.html\n");

    let dur = bench_register_flow(5_000);
    println!("粗基准: 5000 次完整注册链路 ≈ {:?}\n", dur);

    run_tests();
}

fn main() {
    bootstrap();
}

/*
// ── 若在 lib crate 中，可新增 tests/integration_test.rs（示例）──
//
// use my_app::{AppHarness, HttpStatus};
//
// #[test]
// fn register_and_get_profile() {
//     let mut app = AppHarness::new();
//     let r = app.post_register("it@example.com");
//     assert_eq!(r.status, HttpStatus::Ok200);
// }
*/