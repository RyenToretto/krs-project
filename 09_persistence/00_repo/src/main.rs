/*
[主题]: Repository 模式 —— trait 抽象持久化 + 内存假实现
[难度]: Easy → Medium
[前置]: `04_traits_generics`、`03_error_collections`、`08_web_axum/03_layers`（分层概念，可选）

一、主题说明
- **Repository（仓储）**：业务层只调用「按 ID 查 / 保存 / 删除」等接口，不知道数据在 HashMap 还是 PostgreSQL。
- 本课用 **trait + InMemory** 实现，等价于生产里 `UserRepository` + `PgUserRepository`（sqlx），但**不连真库**。
- 好处：单元测试秒级、CI 离线；换存储只换 impl，不动 `OrderService`。

二、写法对比
1. 直观版: `OrderService` 内部直接持有 `HashMap<u64, User>`，测试与真库代码缠在一起。
2. 进阶版: 定义 `UserRepository` trait，`InMemoryUserRepo` 与未来的 `SqlxUserRepo` 都 `impl` 它。

三、真实用途与 sqlx 对照
- Axum Handler → Service → **Repository trait** → sqlx `Pool` 执行 SQL（见 `09_persistence/01_sqlx`）。
- serde 常用来把 DB 行映射成结构体；本课用手写 struct 即可，serde 见 https://serde.rs/
资料: https://docs.rs/sqlx/latest/sqlx/
      https://github.com/launchbadge/sqlx
      https://serde.rs/
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;
use std::fmt;

// ── 领域模型 ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
struct User {
    id: u64,
    email: String,
}

// ── 仓储错误（类似 sqlx::Error 的简化版）──────────────────────────

#[derive(Debug, PartialEq)]
enum RepoError {
  NotFound(u64),
  DuplicateEmail(String),
  Internal(String),
}

impl fmt::Display for RepoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepoError::NotFound(id) => write!(f, "用户 {} 不存在", id),
            RepoError::DuplicateEmail(e) => write!(f, "邮箱已占用: {}", e),
            RepoError::Internal(msg) => write!(f, "内部错误: {}", msg),
        }
    }
}

// ── 进阶：Repository trait ──────────────────────────────────────

/// 用户仓储契约 —— 真项目里会有 `async fn` + `sqlx::Pool`，这里用同步 trait 降低门槛
trait UserRepository {
    fn find_by_id(&self, id: u64) -> Result<Option<User>, RepoError>;
    fn find_by_email(&self, email: &str) -> Result<Option<User>, RepoError>;
    fn insert(&mut self, user: User) -> Result<(), RepoError>;
    fn update(&mut self, user: User) -> Result<(), RepoError>;
    fn delete(&mut self, id: u64) -> Result<bool, RepoError>;
    fn list(&self) -> Result<Vec<User>, RepoError>;
}

/// 内存假实现：行为像一张 `users` 表，主键 `id`，`email` 唯一
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
    fn find_by_id(&self, id: u64) -> Result<Option<User>, RepoError> {
        Ok(self.by_id.get(&id).cloned())
    }

    fn find_by_email(&self, email: &str) -> Result<Option<User>, RepoError> {
        let key = email.to_lowercase();
        match self.by_email.get(&key) {
            Some(id) => self.find_by_id(*id),
            None => Ok(None),
        }
    }

    fn insert(&mut self, user: User) -> Result<(), RepoError> {
        let key = user.email.to_lowercase();
        if self.by_email.contains_key(&key) {
            return Err(RepoError::DuplicateEmail(user.email));
        }
        self.by_email.insert(key, user.id);
        self.by_id.insert(user.id, user);
        Ok(())
    }

    fn update(&mut self, user: User) -> Result<(), RepoError> {
        if !self.by_id.contains_key(&user.id) {
            return Err(RepoError::NotFound(user.id));
        }
        let key = user.email.to_lowercase();
        if let Some(&existing_id) = self.by_email.get(&key) {
            if existing_id != user.id {
                return Err(RepoError::DuplicateEmail(user.email));
            }
        }
        // 若改了邮箱，先清旧映射
        if let Some(old) = self.by_id.get(&user.id) {
            let old_key = old.email.to_lowercase();
            if old_key != key {
                self.by_email.remove(&old_key);
                self.by_email.insert(key, user.id);
            }
        }
        self.by_id.insert(user.id, user);
        Ok(())
    }

    fn delete(&mut self, id: u64) -> Result<bool, RepoError> {
        match self.by_id.remove(&id) {
            Some(u) => {
                self.by_email.remove(&u.email.to_lowercase());
                Ok(true)
            }
            None => Ok(false),
        }
    }

    fn list(&self) -> Result<Vec<User>, RepoError> {
        let mut v: Vec<User> = self.by_id.values().cloned().collect();
        v.sort_by_key(|u| u.id);
        Ok(v)
    }
}

// ── 业务服务：只依赖 trait，不依赖 HashMap ───────────────────────

struct UserService<'a, R: UserRepository + ?Sized> {
    repo: &'a mut R,
}

impl<'a, R: UserRepository + ?Sized> UserService<'a, R> {
    fn register(&mut self, id: u64, email: impl Into<String>) -> Result<User, RepoError> {
        let email = email.into();
        if self.repo.find_by_email(&email)?.is_some() {
            return Err(RepoError::DuplicateEmail(email));
        }
        let user = User { id, email };
        self.repo.insert(user.clone())?;
        Ok(user)
    }

    fn rename_email(&mut self, id: u64, new_email: impl Into<String>) -> Result<User, RepoError> {
        let mut user = self
            .repo
            .find_by_id(id)?
            .ok_or(RepoError::NotFound(id))?;
        user.email = new_email.into();
        self.repo.update(user.clone())?;
        Ok(user)
    }
}

// ── 直观版：Service 直接摸 HashMap（难测、难换存储）──────────────

struct NaiveUserService {
    users: HashMap<u64, User>,
}

impl NaiveUserService {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    fn save_naive(&mut self, user: User) {
        self.users.insert(user.id, user);
    }

    fn get_naive(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
}

fn run_tests() {
    // 直观版
    let mut naive = NaiveUserService::new();
    naive.save_naive(User {
        id: 1,
        email: "a@x.com".into(),
    });
    assert_eq!(naive.get_naive(1).unwrap().email, "a@x.com");

    // 进阶版 + InMemory
    let mut repo = InMemoryUserRepo::new();
    {
        let mut svc = UserService { repo: &mut repo };
        let u = svc.register(10, "learner@rust.dev").unwrap();
        assert_eq!(u.id, 10);
        let again = svc.register(11, "learner@rust.dev");
        assert_eq!(
            again,
            Err(RepoError::DuplicateEmail("learner@rust.dev".into()))
        );
    }
  {
        let mut svc = UserService { repo: &mut repo };
        let updated = svc.rename_email(10, "new@rust.dev").unwrap();
        assert_eq!(updated.email, "new@rust.dev");
    }
    assert!(repo.find_by_id(999).unwrap().is_none());
    assert_eq!(repo.delete(10).unwrap(), true);
    assert_eq!(repo.list().unwrap().len(), 0);

    let display = format!("{}", RepoError::Internal("模拟连接断开".into()));
    assert!(display.contains("内部错误"));

    println!("✅ Repository trait + InMemory 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Repository trait ************\n");

    let mut repo = InMemoryUserRepo::new();
    let mut svc = UserService { repo: &mut repo };

    match svc.register(1, "demo@krs.dev") {
        Ok(u) => println!("注册成功: {:?}", u),
        Err(e) => println!("注册失败: {}", e),
    }

    let mut naive = NaiveUserService::new();
    naive.save_naive(User {
        id: 99,
        email: "naive@example.com".into(),
    });
    println!(
        "直观版查到: {:?}",
        naive.get_naive(99).map(|u| u.email.as_str())
    );

    println!("\n对照生产: 把 InMemoryUserRepo 换成持 sqlx::Pool 的类型，Service 签名不变。");
    println!("sqlx 文档: https://docs.rs/sqlx/latest/sqlx/\n");

    run_tests();
}

fn main() {
    bootstrap();
}