/*
[主题]: 服务拆分 —— 从单体到边界清晰的微服务（内存 RPC，无真网络）
[难度]: Medium
[前置]: `08_web_axum/03_layers`（Handler/Service 分层）、`09_persistence/00_repo`（trait 抽象）

一、主题说明
- **单体**：用户、订单、库存逻辑挤在一个进程里，改一处牵全身，部署只能「全量发版」。
- **拆分**：按**限界上下文**拆成独立「服务」——各自有数据结构、错误类型、存储；之间只通过**契约**（本课用 trait + 消息枚举）通信。
- 真环境常用 **gRPC + tonic** 调远程服务；本课用 `InProcessBus` 模拟「进程内仍可先拆模块、后再上 RPC」的路径。
- 拆分原则（入门记三条）：**单一职责**、**数据归属清晰**、**跨服务不共享可变状态**。

二、写法对比
1. 直观版: `Monolith` 一个 struct 里 `users` + `orders` 两个 HashMap，方法互相直接改对方数据。
2. 进阶版: `UserService` / `OrderService` 独立 crate 级模块 + `ServiceRegistry` 注册；下单通过 `OrderService` 调 `UserClient` 校验用户存在。

三、真实用途与 tonic 对照
- 单体拆完后，把 `UserClient` trait 的实现从 `InProcessBus` 换成 **tonic 生成的 `UserServiceClient`** 即可，业务编排代码几乎不动。
- proto 文件定义 `GetUser`、`CreateOrder` RPC；每服务一个二进制，K8s 独立扩缩。
资料: https://docs.rs/tonic/latest/tonic/
      https://github.com/hyperium/tonic
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;

// ── 共享「消息」：类似 proto 里的 request/response（本课手写，不用 prost）────

#[derive(Clone, Debug, PartialEq, Eq)]
struct UserDto {
    id: u64,
    email: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OrderDto {
    id: u64,
    user_id: u64,
    product: String,
    cents: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ServiceError {
    UserNotFound(u64),
    DuplicateEmail(String),
    InvalidOrder(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::UserNotFound(id) => write!(f, "用户不存在: {}", id),
            ServiceError::DuplicateEmail(e) => write!(f, "邮箱重复: {}", e),
            ServiceError::InvalidOrder(m) => write!(f, "订单无效: {}", m),
        }
    }
}

// ── 用户服务边界 ───────────────────────────────────────────────────

struct UserService {
    users: HashMap<u64, UserDto>,
    emails: HashMap<String, u64>,
    next_id: u64,
}

impl UserService {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            emails: HashMap::new(),
            next_id: 1,
        }
    }

    fn create_user(&mut self, email: impl Into<String>) -> Result<UserDto, ServiceError> {
        let email = email.into();
        let key = email.to_lowercase();
        if self.emails.contains_key(&key) {
            return Err(ServiceError::DuplicateEmail(email));
        }
        let id = self.next_id;
        self.next_id += 1;
        let u = UserDto { id, email: email.clone() };
        self.emails.insert(key, id);
        self.users.insert(id, u.clone());
        Ok(u)
    }

    fn get_user(&self, id: u64) -> Result<UserDto, ServiceError> {
        self.users
            .get(&id)
            .cloned()
            .ok_or(ServiceError::UserNotFound(id))
    }

    fn exists(&self, id: u64) -> bool {
        self.users.contains_key(&id)
    }
}

// ── 订单服务边界（不直接摸 UserService 的 HashMap）──────────────────

/// 用户服务对外的「客户端契约」—— 生产中由 tonic stub 实现
trait UserLookup {
    fn user_exists(&self, user_id: u64) -> bool;
}

struct OrderService<'a, U: UserLookup + ?Sized> {
    orders: HashMap<u64, OrderDto>,
    next_id: u64,
    users: &'a U,
}

impl<'a, U: UserLookup + ?Sized> OrderService<'a, U> {
    fn new(users: &'a U) -> Self {
        Self {
            orders: HashMap::new(),
            next_id: 1000,
            users,
        }
    }

    fn place_order(
        &mut self,
        user_id: u64,
        product: impl Into<String>,
        cents: u64,
    ) -> Result<OrderDto, ServiceError> {
        if !self.users.user_exists(user_id) {
            return Err(ServiceError::UserNotFound(user_id));
        }
        if cents == 0 {
            return Err(ServiceError::InvalidOrder("金额不能为 0".into()));
        }
        let id = self.next_id;
        self.next_id += 1;
        let o = OrderDto {
            id,
            user_id,
            product: product.into(),
            cents,
        };
        self.orders.insert(id, o.clone());
        Ok(o)
    }

    fn list_by_user(&self, user_id: u64) -> Vec<OrderDto> {
        let mut v: Vec<_> = self
            .orders
            .values()
            .filter(|o| o.user_id == user_id)
            .cloned()
            .collect();
        v.sort_by_key(|o| o.id);
        v
    }
}

// ── 进阶：进程内「总线」+ 注册表（模拟多服务部署前的一步）──────────

struct InProcessBus {
    user: UserService,
}

impl UserLookup for InProcessBus {
    fn user_exists(&self, user_id: u64) -> bool {
        self.user.exists(user_id)
    }
}

struct ServiceRegistry {
    /// 服务名 → 是否已注册（真环境里是 Consul/etcd + 健康检查）
    names: HashMap<String, bool>,
}

impl ServiceRegistry {
    fn new() -> Self {
        Self {
            names: HashMap::new(),
        }
    }

    fn register(&mut self, name: impl Into<String>) {
        self.names.insert(name.into(), true);
    }

    fn is_registered(&self, name: &str) -> bool {
        self.names.get(name).copied().unwrap_or(false)
    }
}

// ── 直观版：未拆分单体 ─────────────────────────────────────────────

struct Monolith {
    users: HashMap<u64, UserDto>,
    orders: HashMap<u64, OrderDto>,
    next_user: u64,
    next_order: u64,
}

impl Monolith {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
            orders: HashMap::new(),
            next_user: 1,
            next_order: 1,
        }
    }

    fn add_user(&mut self, email: &str) -> u64 {
        let id = self.next_user;
        self.next_user += 1;
        self.users.insert(id, UserDto { id, email: email.into() });
        id
    }

    fn add_order(&mut self, user_id: u64, product: &str) {
        // 直观版：直接读 users HashMap，模块边界模糊
        if !self.users.contains_key(&user_id) {
            return;
        }
        let id = self.next_order;
        self.next_order += 1;
        self.orders.insert(
            id,
            OrderDto {
                id,
                user_id,
                product: product.into(),
                cents: 100,
            },
        );
    }

    fn order_count(&self) -> usize {
        self.orders.len()
    }
}

fn run_tests() {
    // 直观版
    let mut mono = Monolith::new();
    let uid = mono.add_user("mono@test.dev");
    mono.add_order(uid, "keyboard");
    assert_eq!(mono.order_count(), 1);

    // 进阶：拆分 + trait 边界
    let mut bus = InProcessBus {
        user: UserService::new(),
    };
    let u = bus.user.create_user("split@krs.dev").unwrap();
    assert_eq!(bus.user.get_user(u.id).unwrap().email, "split@krs.dev");
    {
        let mut orders = OrderService::new(&bus);
        let o = orders.place_order(u.id, "rust-book", 4999).unwrap();
        assert_eq!(o.user_id, u.id);
        assert_eq!(orders.list_by_user(u.id).len(), 1);
    }
    let bad = {
        let mut orders = OrderService::new(&bus);
        orders.place_order(9999, "ghost", 100)
    };
    assert_eq!(bad, Err(ServiceError::UserNotFound(9999)));

    let dup = bus.user.create_user("split@krs.dev");
    assert!(matches!(dup, Err(ServiceError::DuplicateEmail(_))));

    let mut reg = ServiceRegistry::new();
    reg.register("user-service");
    reg.register("order-service");
    assert!(reg.is_registered("user-service"));
    assert!(!reg.is_registered("payment-service"));

    println!("✅ 服务拆分（单体 vs 边界 + trait 客户端）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 服务拆分 ************\n");

    let mut mono = Monolith::new();
    println!(
        "直观版单体: 用户+订单同 struct，订单数={}",
        {
            let uid = mono.add_user("demo@mono.dev");
            mono.add_order(uid, "mouse");
            mono.order_count()
        }
    );

    let mut bus = InProcessBus {
        user: UserService::new(),
    };
    let user = bus.user.create_user("demo@split.dev").expect("注册");
    let mut orders = OrderService::new(&bus);
    match orders.place_order(user.id, "tonic-course", 0) {
        Err(e) => println!("进阶版校验: {}", e),
        Ok(o) => println!("下单: {:?}", o),
    }
    match orders.place_order(user.id, "tonic-course", 2900) {
        Ok(o) => println!("下单成功: order_id={} product={}", o.id, o.product),
        Err(e) => println!("失败: {}", e),
    }

    println!("\n对照 tonic: UserLookup → generated Client；UserService/OrderService → 独立 tonic Server。");
    println!("文档: https://docs.rs/tonic/latest/tonic/\n");

    run_tests();
}

fn main() {
    bootstrap();
}