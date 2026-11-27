/*
[主题]: 云原生配置与密钥 —— env 注入、挂载 Secret、日志脱敏（离线可测）
[难度]: Medium → Hard
[前置]: `16_production_rust/01_config`（分层配置）、`14_observability`（结构化日志心智）

一、主题说明
- 云原生常见组合：**非敏感**走 ConfigMap/env；**敏感**走 Secret（挂载文件或 env，仍可能被进程看见）。
- **12-Factor**：配置存环境；**K8s** 又常把 Secret 挂到 `/var/run/secrets/...` 只读卷。
- **陷阱**：`println!("db={}", url)`、`Debug` 打印整 struct、`RUST_LOG=trace` 打到收集器仍含 token。

二、写法对比
1. 直观版: `env::var("API_KEY").unwrap()` 后直接拼接日志。
2. 进阶版: `SecretString` + `CloudConfig::load` + `StructuredEvent` 字段级脱敏（对照 `tracing` 的 `%` Display）。

三、真实用途与生产对照
| 本课 | 平台 |
| --- | --- |
| `SecretMount` | K8s `volumeMount` Secret |
| `EnvSecretLayer` | `envFrom.secretRef` |
| `Redacted` / `SecretString` | `secrecy` crate |
| `StructuredEvent` | `tracing::info!(key = %redacted, ...)` |

资料:
https://doc.rust-lang.org/book/
https://docs.rs/tracing/latest/tracing/
https://doc.rust-lang.org/embedded-book/
https://www.rust-lang.org/what/embedded
*/

use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;

// ══════════════════════════════════════════════════════════════════
// 秘密类型：Debug 也不泄漏（对照 secrecy::SecretBox）
// ══════════════════════════════════════════════════════════════════

#[derive(Clone)]
struct SecretString(String);

impl SecretString {
    fn new(raw: String) -> Self {
        Self(raw)
    }

    fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretString(***)")
    }
}

#[derive(Clone)]
struct Redacted<'a>(pub &'a str);

impl fmt::Display for Redacted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.0;
        if s.len() <= 6 {
            write!(f, "***")
        } else {
            write!(f, "{}***", &s[..3.min(s.len())])
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// 云原生配置模型
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
struct CloudConfig {
    service_name: String,
    listen_port: u16,
    /// 非敏感
    log_level: String,
    /// 敏感：数据库 URL 含密码
    database_url: SecretString,
    /// 敏感：API 密钥
    api_key: SecretString,
}

#[derive(Debug, PartialEq, Eq)]
enum LoadError {
    Missing(&'static str),
    ParsePort(String),
    Io(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Missing(k) => write!(f, "缺少: {k}"),
            LoadError::ParsePort(raw) => write!(f, "端口非法: {raw}"),
            LoadError::Io(e) => write!(f, "IO: {e}"),
        }
    }
}

/// 模拟 K8s Secret 卷：key -> 文件内容
#[derive(Clone, Debug)]
struct SecretMount {
    base_dir: String,
    files: HashMap<String, String>,
}

impl SecretMount {
    fn in_memory(base: &str, files: HashMap<String, String>) -> Self {
        Self {
            base_dir: base.into(),
            files,
        }
    }

    fn read_key(&self, filename: &'static str) -> Result<String, LoadError> {
        self.files
            .get(filename)
            .cloned()
            .ok_or(LoadError::Missing(filename))
    }

    /// 对照真实挂载：从磁盘读（测试用 tempdir）
    fn read_key_from_fs(&self, filename: &'static str) -> Result<String, LoadError> {
        let path = format!("{}/{}", self.base_dir, filename);
        fs::read_to_string(&path).map_err(|e| LoadError::Io(e.to_string()))
    }
}

struct EnvLayer;

impl EnvLayer {
    fn apply(map: &mut HashMap<&'static str, String>) {
        const KEYS: &[&str] = &[
            "SERVICE_NAME",
            "LISTEN_PORT",
            "LOG_LEVEL",
            "DATABASE_URL",
            "API_KEY",
        ];
        for k in KEYS {
            if let Ok(v) = env::var(k) {
                map.insert(match *k {
                    "SERVICE_NAME" => "service_name",
                    "LISTEN_PORT" => "listen_port",
                    "LOG_LEVEL" => "log_level",
                    "DATABASE_URL" => "database_url",
                    "API_KEY" => "api_key",
                    _ => continue,
                }, v);
            }
        }
    }
}

fn parse_port(raw: &str) -> Result<u16, LoadError> {
    raw.parse::<u16>()
        .map_err(|_| LoadError::ParsePort(raw.into()))
}

fn build_config(map: &HashMap<&'static str, String>) -> Result<CloudConfig, LoadError> {
    let service_name = map
        .get("service_name")
        .cloned()
        .unwrap_or_else(|| "unknown".into());
    let port_raw = map.get("listen_port").map(|s| s.as_str()).unwrap_or("8080");
    let listen_port = parse_port(port_raw)?;
    let log_level = map
        .get("log_level")
        .cloned()
        .unwrap_or_else(|| "info".into());
    let db = map
        .get("database_url")
        .cloned()
        .ok_or(LoadError::Missing("database_url"))?;
    let api_key = map
        .get("api_key")
        .cloned()
        .ok_or(LoadError::Missing("api_key"))?;
    Ok(CloudConfig {
        service_name,
        listen_port,
        log_level,
        database_url: SecretString::new(db),
        api_key: SecretString::new(api_key),
    })
}

/// 进阶加载：默认 → Secret 卷 → env 覆盖（env 优先级最高）
fn load_cloud_config(mount: &SecretMount) -> Result<CloudConfig, LoadError> {
    let mut map: HashMap<&'static str, String> = HashMap::new();
    map.insert("service_name", "billing-api".into());
    map.insert("listen_port", "8080".into());
    map.insert("log_level", "info".into());

    let db_from_mount = mount.read_key("database_url")?;
    let key_from_mount = mount.read_key("api_key")?;
    map.insert("database_url", db_from_mount);
    map.insert("api_key", key_from_mount);

    EnvLayer::apply(&mut map);
    build_config(&map)
}

impl CloudConfig {
    fn safe_summary(&self) -> String {
        format!(
            "service={} port={} log={} db={} api_key={}",
            self.service_name,
            self.listen_port,
            self.log_level,
            Redacted(self.database_url.expose()),
            Redacted(self.api_key.expose()),
        )
    }
}

// ══════════════════════════════════════════════════════════════════
// 结构化日志（对照 tracing field display，无第三方依赖）
// ══════════════════════════════════════════════════════════════════

struct StructuredEvent {
    target: &'static str,
    fields: Vec<(&'static str, String)>,
}

impl StructuredEvent {
    fn info(target: &'static str) -> Self {
        Self {
            target,
            fields: Vec::new(),
        }
    }

    fn field(mut self, key: &'static str, value: impl fmt::Display) -> Self {
        self.fields.push((key, value.to_string()));
        self
    }

    fn field_secret(mut self, key: &'static str, secret: &SecretString) -> Self {
        self.fields
            .push((key, Redacted(secret.expose()).to_string()));
        self
    }

    fn render(&self) -> String {
        let body: Vec<String> = self
            .fields
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        format!("INFO {} {}", self.target, body.join(" "))
    }
}

// ══════════════════════════════════════════════════════════════════
// 直观版反模式
// ══════════════════════════════════════════════════════════════════

fn naive_log_config(cfg: &CloudConfig) -> String {
    // 陷阱：秘密明文进日志
    format!(
        "starting service={} db={} key={}",
        cfg.service_name,
        cfg.database_url.expose(),
        cfg.api_key.expose(),
    )
}

// ══════════════════════════════════════════════════════════════════
// 测试辅助
// ══════════════════════════════════════════════════════════════════

struct EnvGuard {
    pairs: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn set(key: &'static str, val: &str) -> Self {
        env::set_var(key, val);
        Self {
            pairs: vec![(key, Some(val.into()))],
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, _) in &self.pairs {
            env::remove_var(k);
        }
    }
}

fn run_tests() {
    let mount = SecretMount::in_memory(
        "/var/run/secrets/app",
        HashMap::from([
            (
                "database_url".into(),
                "postgres://app:SuperSecret@db.internal/app".into(),
            ),
            ("api_key".into(), "sk-live-abcdefgh123456".into()),
        ]),
    );

    let cfg = load_cloud_config(&mount).expect("默认+挂载应成功");
    assert_eq!(cfg.service_name, "billing-api");
    assert_eq!(cfg.listen_port, 8080);

    let summary = cfg.safe_summary();
    assert!(
        !summary.contains("SuperSecret"),
        "摘要不得含 db 密码: {summary}"
    );
    assert!(
        !summary.contains("sk-live-abcdefgh123456"),
        "摘要不得含 api_key"
    );
    assert!(summary.contains("***"));

    let event = StructuredEvent::info("bootstrap")
        .field("service", &cfg.service_name)
        .field_secret("api_key", &cfg.api_key);
    let rendered = event.render();
    assert!(!rendered.contains("sk-live"));
    assert!(rendered.contains("api_key="));

    let naive = naive_log_config(&cfg);
    assert!(naive.contains("SuperSecret"), "反模式应暴露泄漏");

    // env 覆盖挂载
    let cfg2 = {
        let _g = EnvGuard::set("LISTEN_PORT", "3001");
        load_cloud_config(&mount).unwrap()
    };
    assert_eq!(cfg2.listen_port, 3001);

    // fs 挂载路径（临时目录）
    let tmp = env::temp_dir().join("krs_secret_test");
    let _ = fs::create_dir_all(&tmp);
    let db_path = tmp.join("database_url");
    fs::write(&db_path, "postgres://mount:pwd@h/db").unwrap();
    let mount_fs = SecretMount {
        base_dir: tmp.to_string_lossy().into(),
        files: HashMap::new(),
    };
    let raw = mount_fs.read_key_from_fs("database_url").unwrap();
    assert!(raw.contains("mount:pwd"));

    let dbg = format!("{:?}", SecretString::new("hidden".into()));
    assert!(!dbg.contains("hidden"));
    assert!(dbg.contains("***"));

    println!("✅ l00_config_secrets 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 配置与密钥：挂载 + env + 脱敏 ************\n");

    let mount = SecretMount::in_memory(
        "/var/run/secrets/app",
        HashMap::from([
            (
                "database_url".into(),
                "postgres://app:localonly@127.0.0.1/dev".into(),
            ),
            ("api_key".into(), "sk-dev-not-for-logs".into()),
        ]),
    );

    match load_cloud_config(&mount) {
        Ok(cfg) => {
            println!("安全摘要: {}", cfg.safe_summary());
            let evt = StructuredEvent::info("cloud_native::config")
                .field("port", cfg.listen_port)
                .field("log_level", &cfg.log_level)
                .field_secret("database_url", &cfg.database_url);
            println!("结构化日志: {}", evt.render());
            println!("（生产用 tracing: https://docs.rs/tracing/latest/tracing/ ）");
        }
        Err(e) => println!("加载失败: {e}"),
    }

    println!("\n--- 反模式 naive_log ---");
    if let Ok(cfg) = load_cloud_config(&mount) {
        println!("切勿这样: {}", naive_log_config(&cfg));
    }

    println!("\n陷阱回顾:");
    println!("- Secret 挂 env 仍可在 /proc/容器内被看见 —— 最小权限 + 轮换");
    println!("- 勿 Derive Debug 含秘密字段；用 SecretString 或 secrecy");
    println!("- 配置与 `16_production_rust/01_config` 分层合并互补：本课强调密钥面");

    run_tests();
}

fn main() {
    bootstrap();
}