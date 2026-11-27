/*
[主题]: 生产错误模型 —— 库边界用 thiserror，二进制边界用 anyhow + context
[难度]: Medium → Hard
[前置]: `03_error_collections`（Result/?）、`15_project_capstone`（AppError 枚举）

一、主题说明
- **分层**：库 / crate 暴露**稳定、可匹配**的错误枚举（类似 `thiserror::Error`）；可执行文件在 `main` 里把任意失败收成 `anyhow::Result`，用 `.context()` 叠调用链。
- **陷阱**：`Box<dyn std::error::Error>` 默认不 `Send`+`Sync`；到处 `String` 错误无法 `match`；在库 public API 里返回 `anyhow::Error` 会把实现细节绑死。
- 本课：**直观版**用手写 `Display`/`source` 模拟 derive；**进阶版**用真实 `thiserror` + `anyhow`。

二、写法对比
1. 直观版: 手写 `DomainError` + `Error::source` 链，应用层 `map_err` 加前缀。
2. 进阶版: `#[derive(thiserror::Error)]` + `anyhow::Context` + `?` 穿透。

三、真实用途与生产对照
| 本课 | 生产组件 |
| --- | --- |
| `DomainError` / `StorageError` | sqlx::Error、`redis::RedisError` 包一层 thiserror |
| `run_app() -> anyhow::Result<()>` | Axum `main`、CLI binary 顶层 |
| `context("加载用户 {id}")` | `tracing` span + anyhow 双层排障 |
| 禁止库导出 `anyhow::Error` | API Guidelines C-GOOD-ERR |

资料: https://docs.rs/thiserror/latest/thiserror/
      https://docs.rs/anyhow/latest/anyhow/
      https://rust-lang.github.io/api-guidelines/
*/

use anyhow::Context;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io;

// ══════════════════════════════════════════════════════════════════
// 一、直观版：手写分层错误（对照 thiserror 会生成什么）
// ══════════════════════════════════════════════════════════════════

/// 领域 / 持久化边界 —— 库里应暴露这类枚举，而非 String
#[derive(Debug, PartialEq, Eq)]
enum DomainErrorNaive {
    UserNotFound(u64),
    InvalidEmail(String),
}

impl fmt::Display for DomainErrorNaive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainErrorNaive::UserNotFound(id) => write!(f, "用户 {id} 不存在"),
            DomainErrorNaive::InvalidEmail(e) => write!(f, "邮箱无效: {e}"),
        }
    }
}

impl Error for DomainErrorNaive {}

#[derive(Debug, PartialEq, Eq)]
enum StorageErrorNaive {
    Io(io::ErrorKind),
    Domain(DomainErrorNaive),
}

impl fmt::Display for StorageErrorNaive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageErrorNaive::Io(kind) => write!(f, "IO 失败: {kind:?}"),
            StorageErrorNaive::Domain(d) => write!(f, "{d}"),
        }
    }
}

impl Error for StorageErrorNaive {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            StorageErrorNaive::Domain(d) => Some(d),
            StorageErrorNaive::Io(_) => None,
        }
    }
}

fn load_user_naive(db: &HashMap<u64, String>, id: u64) -> Result<String, StorageErrorNaive> {
    if id == 0 {
        return Err(StorageErrorNaive::Domain(DomainErrorNaive::InvalidEmail(
            "id 不能为 0".into(),
        )));
    }
    db.get(&id)
        .cloned()
        .ok_or(StorageErrorNaive::Domain(DomainErrorNaive::UserNotFound(id)))
}

/// **陷阱演示**：`Box<dyn Error>` 不带 `Send`，难塞进 `tokio::spawn`
fn naive_box_trap() -> Result<(), Box<dyn Error>> {
    Err(StorageErrorNaive::Domain(DomainErrorNaive::UserNotFound(9)).into())
}

// ══════════════════════════════════════════════════════════════════
// 二、进阶版：thiserror + anyhow（库 / bin 分界）
// ══════════════════════════════════════════════════════════════════

#[derive(Debug, thiserror::Error, PartialEq)]
enum DomainError {
    #[error("用户 {0} 不存在")]
    UserNotFound(u64),

    #[error("邮箱无效: {0}")]
    InvalidEmail(String),
}

#[derive(Debug, thiserror::Error, PartialEq)]
enum StorageError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("数据库 IO: {0}")]
    Io(String),
}

#[derive(Debug, thiserror::Error)]
#[error("HTTP 层错误，状态 {status}")]
struct ApiError {
    status: u16,
    #[source]
    inner: StorageError,
}

impl ApiError {
    fn not_found_from_storage(err: StorageError) -> Self {
        let status = match &err {
            StorageError::Domain(DomainError::UserNotFound(_)) => 404,
            StorageError::Domain(DomainError::InvalidEmail(_)) => 400,
            StorageError::Io(_) => 503,
        };
        Self {
            status,
            inner: err,
        }
    }
}

fn load_user(db: &HashMap<u64, String>, id: u64) -> Result<String, StorageError> {
    if id == 0 {
        return Err(DomainError::InvalidEmail("id 不能为 0".into()).into());
    }
    db.get(&id)
        .cloned()
        .ok_or(DomainError::UserNotFound(id).into())
}

/// 二进制边界：业务 + 上下文，返回 anyhow
fn handle_get_user(
    db: &HashMap<u64, String>,
    id: u64,
) -> anyhow::Result<(u16, String)> {
    let body = load_user(db, id)?;
    Ok((200, body))
}

fn run_http_stack(db: &HashMap<u64, String>, id: u64) -> anyhow::Result<String> {
    let (status, body) = handle_get_user(db, id)
        .with_context(|| format!("处理 GET /users/{id}"))?;
    if status != 200 {
        anyhow::bail!("意外状态码 {status}");
    }
    Ok(body)
}

// ══════════════════════════════════════════════════════════════════
// 三、陷阱清单（注释 + 断言形态）
// ══════════════════════════════════════════════════════════════════

struct ErrorLayerTrap;

impl ErrorLayerTrap {
    /// 库 crate 不应把 anyhow 冒泡到 public 函数签名
    fn library_api_smell() -> &'static str {
        "// pub fn parse_token(s: &str) -> anyhow::Result<Claims>  // ❌"
    }

    /// `String` 错误无法区分 404 vs 500
    fn string_error_smell() -> Result<(), String> {
        Err("用户不存在".into())
    }
}

fn format_error_chain(mut err: &dyn Error) -> Vec<String> {
    let mut v = Vec::new();
    loop {
        v.push(err.to_string());
        match err.source() {
            Some(s) => err = s,
            None => break,
        }
    }
    v
}

fn run_tests() {
    // ── 直观版 ──
    let mut db = HashMap::new();
    db.insert(1, "alice@example.com".into());

    assert_eq!(
        load_user_naive(&db, 1).unwrap(),
        "alice@example.com",
        "naive 应能读出用户"
    );
    assert_eq!(
        load_user_naive(&db, 99),
        Err(StorageErrorNaive::Domain(DomainErrorNaive::UserNotFound(99))),
        "naive 缺失用户"
    );

    let chain = format_error_chain(&StorageErrorNaive::Domain(DomainErrorNaive::UserNotFound(7)));
    assert!(
        chain.len() >= 2,
        "手写 Error::source 应形成链，got {:?}",
        chain
    );
    assert!(naive_box_trap().is_err(), "Box<dyn Error> 演示");

    // ── 进阶 thiserror ──
    assert_eq!(
        load_user(&db, 1).unwrap(),
        "alice@example.com",
        "thiserror 版读用户"
    );
    let bad: StorageError = DomainError::InvalidEmail("x".into()).into();
    assert!(
        matches!(bad, StorageError::Domain(DomainError::InvalidEmail(_))),
        "From 派生应生效"
    );

    let api = ApiError::not_found_from_storage(DomainError::UserNotFound(42).into());
    assert_eq!(api.status, 404, "领域错误应映射 HTTP 404");

    // ── anyhow context ──
    let ok = run_http_stack(&db, 1).unwrap();
    assert_eq!(ok, "alice@example.com");

    let missing = run_http_stack(&db, 404);
    assert!(missing.is_err(), "缺失用户应失败");
    let msg = format!("{:?}", missing.unwrap_err());
    assert!(
        msg.contains("GET /users/404") || msg.contains("404"),
        "context 应带路由信息: {msg}"
    );

    let trap = ErrorLayerTrap::string_error_smell();
    assert!(trap.is_err());
    assert_eq!(
        ErrorLayerTrap::library_api_smell(),
        "// pub fn parse_token(s: &str) -> anyhow::Result<Claims>  // ❌"
    );

    println!("✅ l00_thiserror_anyhow 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 生产错误：thiserror + anyhow ************\n");

    let mut db = HashMap::new();
    db.insert(1, "ops@prod.example".into());

    println!("--- 直观版 load_user_naive ---");
    match load_user_naive(&db, 1) {
        Ok(u) => println!("OK: {u}"),
        Err(e) => println!("ERR: {e} (source: {:?})", e.source()),
    }

    println!("\n--- 进阶版 run_http_stack + context ---");
    match run_http_stack(&db, 2) {
        Ok(u) => println!("OK: {u}"),
        Err(e) => println!("排障链:\n{e:?}"),
    }

    println!("\n--- 对照生产 ---");
    println!("库: #[derive(thiserror::Error)] + #[from] 代替手写 source");
    println!("bin: fn main() -> anyhow::Result<()> {{ ... }}");
    println!("HTTP: map StorageError → axum::response::IntoResponse（本课用 ApiError.status）");

    run_tests();
}

fn main() {
    bootstrap();
}