/*
[主题]: 配置心智 —— 默认值 → 文件 → 环境变量，校验与脱敏
[难度]: Medium → Hard
[前置]: `04_generics_traits`（trait）、`03_error_collections`（Result）

一、主题说明
- 生产服务 seldom 只有单一配置源：**默认** + **文件**（toml/yaml）+ **环境变量**覆盖；最后做**类型化校验**（端口范围、必填项、互斥开关）。
- **陷阱**：启动时 `unwrap` 配置文件；把 `DATABASE_URL` 打进 `println!`；布尔用字符串 `"false"` 却当 true。
- 本课纯 **std** 模拟 `config` / `figment` / `dotenvy` 的分层 merge，不读真磁盘（内嵌 TOML 字符串）。

二、写法对比
1. 直观版: 一个大 struct + `from_env()` 里散落 `std::env::var`。
2. 进阶版: `ConfigLayer` trait、`merge` 顺序固定、`validate()` + `Redacted` 打印。

三、真实用途与生产对照
| 本课 | 生产组件 |
| --- | --- |
| `DefaultLayer` | 代码内 `const` / `Default` |
| `FileLayer` | `config::File::with_name` / `figment::Toml::file` |
| `EnvLayer` | `APP__DATABASE__URL`（serde 双下划线） |
| `AppConfig::validate` | `validator` crate / 启动时 bail |
| `Display for Redacted` | 日志里 `tracing::field::display` 打码 |

资料: https://docs.rs/config/latest/config/
      https://docs.rs/figment/latest/figment/
      https://rust-lang.github.io/api-guidelines/
*/

use std::collections::HashMap;
use std::env;
use std::fmt;

// ══════════════════════════════════════════════════════════════════
// 配置模型
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Debug, PartialEq, Eq)]
struct AppConfig {
    listen_addr: String,
    port: u16,
    db_url: String,
    enable_metrics: bool,
    max_connections: u32,
}

#[derive(Debug, PartialEq, Eq)]
enum ConfigError {
    MissingKey(&'static str),
    ParseInt { key: &'static str, raw: String },
    Validation(&'static str),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingKey(k) => write!(f, "缺少配置项: {k}"),
            ConfigError::ParseInt { key, raw } => write!(f, "{key} 无法解析为整数: {raw}"),
            ConfigError::Validation(msg) => write!(f, "校验失败: {msg}"),
        }
    }
}

/// 日志打印时隐藏秘密（对照 `secrecy::SecretString`）
#[derive(Clone)]
struct Redacted(String);

impl fmt::Display for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.len() <= 8 {
            write!(f, "***")
        } else {
            write!(f, "{}***{}", &self.0[..4], &self.0[self.0.len() - 2..])
        }
    }
}

impl AppConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.port == 0 {
            return Err(ConfigError::Validation("port 不能为 0"));
        }
        if self.port < 1024 && !cfg!(test) {
            // 课内演示：测试允许低位端口
        }
        if self.max_connections == 0 {
            return Err(ConfigError::Validation("max_connections 至少为 1"));
        }
        if !self.db_url.starts_with("postgres://") && !self.db_url.starts_with("sqlite://") {
            return Err(ConfigError::Validation("db_url 前缀非法"));
        }
        Ok(())
    }

    fn redacted_summary(&self) -> String {
        format!(
            "listen={}:{} metrics={} db={} max_conn={}",
            self.listen_addr,
            self.port,
            self.enable_metrics,
            Redacted(self.db_url.clone()),
            self.max_connections,
        )
    }
}

// ══════════════════════════════════════════════════════════════════
// 直观版：散落 env（反模式示范）
// ══════════════════════════════════════════════════════════════════

fn from_env_naive() -> Result<AppConfig, ConfigError> {
    // **陷阱**：顺序不稳；没默认值；布尔解析错
    let port_str = env::var("APP_PORT").map_err(|_| ConfigError::MissingKey("APP_PORT"))?;
    let port: u16 = port_str
        .parse()
        .map_err(|_| ConfigError::ParseInt {
            key: "APP_PORT",
            raw: port_str,
        })?;
    let enable = env::var("APP_ENABLE_METRICS")
        .unwrap_or_else(|_| "true".into())
        .parse::<bool>()
        .unwrap_or(true); // 陷阱：解析失败当 true
    Ok(AppConfig {
        listen_addr: env::var("APP_LISTEN").unwrap_or_else(|_| "0.0.0.0".into()),
        port,
        db_url: env::var("APP_DATABASE_URL")
            .map_err(|_| ConfigError::MissingKey("APP_DATABASE_URL"))?,
        enable_metrics: enable,
        max_connections: 10,
    })
}

// ══════════════════════════════════════════════════════════════════
// 进阶版：分层 ConfigLayer（对照 config / figment merge）
// ══════════════════════════════════════════════════════════════════

trait ConfigLayer {
    fn apply(&self, target: &mut HashMap<&'static str, String>);
}

struct DefaultLayer;

impl ConfigLayer for DefaultLayer {
    fn apply(&self, target: &mut HashMap<&'static str, String>) {
        target.insert("listen", "127.0.0.1".into());
        target.insert("port", "8080".into());
        target.insert("db_url", "sqlite://local.db".into());
        target.insert("enable_metrics", "true".into());
        target.insert("max_connections", "32".into());
    }
}

struct FileLayer {
    /// 极简 TOML 子集：`key = value` 行
    content: &'static str,
}

impl ConfigLayer for FileLayer {
    fn apply(&self, target: &mut HashMap<&'static str, String>) {
        for line in self.content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            target.insert(k.trim(), v.trim().trim_matches('"').to_string());
        }
    }
}

struct EnvLayer {
    prefix: &'static str,
}

impl EnvLayer {
    fn get(&self, short: &'static str) -> Option<String> {
        let key = format!("{}_{}", self.prefix, short.to_uppercase());
        env::var(key).ok()
    }
}

impl ConfigLayer for EnvLayer {
    fn apply(&self, target: &mut HashMap<&'static str, String>) {
        for key in [
            "listen",
            "port",
            "db_url",
            "enable_metrics",
            "max_connections",
        ] {
            if let Some(v) = self.get(key) {
                target.insert(key, v);
            }
        }
    }
}

fn parse_bool(raw: &str) -> Result<bool, ConfigError> {
    match raw.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(ConfigError::Validation("布尔配置仅接受 true/false/0/1")),
    }
}

fn build_config(layers: &[&dyn ConfigLayer]) -> Result<AppConfig, ConfigError> {
    let mut map: HashMap<&'static str, String> = HashMap::new();
    for layer in layers {
        layer.apply(&mut map);
    }

    let port: u16 = map
        .get(&"port")
        .ok_or(ConfigError::MissingKey("port"))?
        .parse()
        .map_err(|_| ConfigError::ParseInt {
            key: "port",
            raw: map.get(&"port").cloned().unwrap_or_default(),
        })?;

    let cfg = AppConfig {
        listen_addr: map
            .get(&"listen")
            .cloned()
            .ok_or(ConfigError::MissingKey("listen"))?,
        port,
        db_url: map
            .get(&"db_url")
            .cloned()
            .ok_or(ConfigError::MissingKey("db_url"))?,
        enable_metrics: parse_bool(
            map.get(&"enable_metrics")
                .ok_or(ConfigError::MissingKey("enable_metrics"))?,
        )?,
        max_connections: map
            .get(&"max_connections")
            .ok_or(ConfigError::MissingKey("max_connections"))?
            .parse()
            .map_err(|_| ConfigError::ParseInt {
                key: "max_connections",
                raw: map.get(&"max_connections").cloned().unwrap_or_default(),
            })?,
    };
    cfg.validate()?;
    Ok(cfg)
}

// ══════════════════════════════════════════════════════════════════
// 测试隔离：临时改 env
// ══════════════════════════════════════════════════════════════════

struct EnvGuard {
    keys: Vec<&'static str>,
}

impl EnvGuard {
    fn set(key: &'static str, val: Option<&str>) -> Self {
        match val {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
        Self { keys: vec![key] }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for k in &self.keys {
            env::remove_var(k);
        }
    }
}

fn run_tests() {
    let file = FileLayer {
        content: r#"
            listen = "0.0.0.0"
            port = 9090
            db_url = "postgres://app:secret@db.internal/app"
            enable_metrics = false
            max_connections = 100
        "#,
    };

    let layers: [&dyn ConfigLayer; 3] = [&DefaultLayer, &file, &EnvLayer { prefix: "APP" }];
    let cfg = build_config(&layers).expect("默认+文件应通过校验");
    assert_eq!(cfg.port, 9090);
    assert!(!cfg.enable_metrics);
    assert!(cfg.db_url.starts_with("postgres://"));

    // env 覆盖 port（限制作用域，避免污染后续用例）
    let cfg2 = {
        let _g = EnvGuard::set("APP_PORT", Some("3000"));
        build_config(&layers).unwrap()
    };
    assert_eq!(cfg2.port, 3000, "环境变量应覆盖文件层");

    let summary = cfg2.redacted_summary();
    assert!(
        !summary.contains("secret"),
        "摘要必须脱敏: {summary}"
    );
    assert!(summary.contains("***"), "应有 Redacted 标记");

    // 校验陷阱
    let bad_file = FileLayer {
        content: "port = 0\ndb_url = \"postgres://x\"\nenable_metrics = true\nmax_connections = 1\nlisten = \"127.0.0.1\"",
    };
    let layers_bad: [&dyn ConfigLayer; 2] = [&bad_file, &EnvLayer { prefix: "APP" }];
    assert!(
        matches!(
            build_config(&layers_bad),
            Err(ConfigError::Validation(_))
        ),
        "port=0 应被拒绝"
    );

    assert!(
        parse_bool("maybe").is_err(),
        "含糊布尔不能默默默认值"
    );

    println!("✅ l00_config_mental 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 配置分层：默认 → 文件 → 环境 ************\n");

    let file = FileLayer {
        content: "port = 8081\ndb_url = \"sqlite://bootstrap.db\"\nenable_metrics = true\nmax_connections = 16\nlisten = \"127.0.0.1\"",
    };
    let layers: [&dyn ConfigLayer; 2] = [&DefaultLayer, &file];
    match build_config(&layers) {
        Ok(c) => println!("启动配置: {}", c.redacted_summary()),
        Err(e) => println!("配置错误: {e}"),
    }

    println!("\n--- 反模式 from_env_naive（仅演示，请勿照搬）---");
    {
        let _p = EnvGuard::set("APP_PORT", Some("8080"));
        let _d = EnvGuard::set("APP_DATABASE_URL", Some("postgres://u:p@h/db"));
        if let Ok(c) = from_env_naive() {
            println!("naive 碰巧成功: {}", c.redacted_summary());
        }
    }

    println!("\n陷阱回顾:");
    println!("- 勿在日志打印完整 DATABASE_URL（用 Redacted / secrecy）");
    println!("- 勿 unwrap 配置；启动失败应 exit 非零并打一条清晰错误");
    println!("- K8s: ConfigMap + Secret 挂载 ↔ 本课 FileLayer + EnvLayer");

    run_tests();
}

fn main() {
    bootstrap();
}