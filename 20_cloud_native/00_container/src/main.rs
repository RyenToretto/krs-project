/*
[主题]: 容器心智 —— 注入环境、只读根文件系统、优雅停机信号（离线模拟，无需真 Docker）
[难度]: Medium → Hard
[前置]: `16_production_rust/01_config`（环境变量）、`03_error_collections`（Result）

一、主题说明
- **容器 ≠ 虚拟机**：进程仍在本机内核上跑，但编排器会给你**固定环境变量**、**只读镜像层 + 可写挂载点**、**SIGTERM 后有限宽限期**再 SIGKILL。
- **PID 1 陷阱**：Rust 二进制若做 init，要处理信号与子进程回收；多数 Rust 服务只做工作进程，由 `tini` / `dumb-init` 或 K8s 包装。
- 本课用纯 **std** 实现 `MiniRuntime`，对照 Docker/K8s 概念，**不**调用容器运行时 API。

二、写法对比
1. 直观版: 直接 `std::env::var` + 任意路径写文件 + `process::exit(0)`，忽略 SIGTERM。
2. 进阶版: `ContainerSpec` 固化 env、写路径白名单、`ShutdownCoordinator` 响应 `Terminate`。

三、真实用途与生产对照
| 本课 | 编排 / 镜像 |
| --- | --- |
| `ContainerSpec::env` | `docker run -e` / K8s `env` / `envFrom` |
| `RootFsPolicy` | 镜像只读层 + `emptyDir` / `volumeMount` 可写目录 |
| `Signal::Terminate` | K8s `terminationGracePeriodSeconds` + SIGTERM |
| `try_write` 拒绝 | 在 `/` 写配置文件的反模式（应写挂载卷） |

资料（云原生可观测常搭配 tracing，本课核心仍用 std 模拟）:
https://doc.rust-lang.org/book/
https://docs.rs/tracing/latest/tracing/
https://doc.rust-lang.org/embedded-book/
https://www.rust-lang.org/what/embedded
*/

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

// ══════════════════════════════════════════════════════════════════
// 信号与停机（对照 Unix SIGTERM / SIGINT）
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Signal {
    /// 编排器要求优雅退出（K8s 删 Pod 时先发 SIGTERM）
    Terminate,
    /// 人为中断（Ctrl+C 心智）
    Interrupt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ShutdownPhase {
    Running,
    Draining { deadline: Duration },
    Exited,
}

struct ShutdownCoordinator {
    grace: Duration,
    phase: ShutdownPhase,
    /// drain 阶段已累计等待（每次 tick 递增）
    drained_for: Duration,
}

impl ShutdownCoordinator {
    fn new(grace: Duration) -> Self {
        Self {
            grace,
            phase: ShutdownPhase::Running,
            drained_for: Duration::ZERO,
        }
    }

    fn on_signal(&mut self, sig: Signal) -> &'static str {
        match self.phase {
            ShutdownPhase::Exited => return "已退出，忽略重复信号",
            ShutdownPhase::Draining { .. } => return "已在 drain，忽略重复信号",
            ShutdownPhase::Running => {}
        }
        match sig {
            Signal::Terminate | Signal::Interrupt => {
                self.drained_for = Duration::ZERO;
                self.phase = ShutdownPhase::Draining {
                    deadline: self.grace,
                };
                "进入 drain：停止接收新连接，等待在途请求"
            }
        }
    }

    fn tick(&mut self, step: Duration) -> bool {
        match &mut self.phase {
            ShutdownPhase::Draining { deadline } => {
                self.drained_for += step;
                if self.drained_for >= *deadline {
                    self.phase = ShutdownPhase::Exited;
                    true
                } else {
                    false
                }
            }
            ShutdownPhase::Exited => true,
            ShutdownPhase::Running => false,
        }
    }

    fn phase_label(&self) -> &'static str {
        match self.phase {
            ShutdownPhase::Running => "Running",
            ShutdownPhase::Draining { .. } => "Draining",
            ShutdownPhase::Exited => "Exited",
        }
    }
}

// ══════════════════════════════════════════════════════════════════
// 只读根 + 可写挂载（对照 OCI 镜像层）
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
struct RootFsPolicy {
    /// 为 true 时，除 `writable_mounts` 外拒绝写入
    read_only_root: bool,
    writable_mounts: Vec<String>,
}

impl RootFsPolicy {
    fn container_default() -> Self {
        Self {
            read_only_root: true,
            writable_mounts: vec!["/tmp".into(), "/var/run".into()],
        }
    }

    fn is_writable(&self, path: &str) -> bool {
        if !self.read_only_root {
            return true;
        }
        self.writable_mounts
            .iter()
            .any(|m| path == m.as_str() || path.starts_with(&format!("{m}/")))
    }
}

#[derive(Debug, PartialEq, Eq)]
enum FsError {
    ReadOnlyRoot { path: String },
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FsError::ReadOnlyRoot { path } => {
                write!(f, "只读根拒绝写入: {path}（请使用 emptyDir/volume 挂载）")
            }
        }
    }
}

fn try_write(path: &str, policy: &RootFsPolicy) -> Result<(), FsError> {
    if policy.is_writable(path) {
        Ok(())
    } else {
        Err(FsError::ReadOnlyRoot {
            path: path.into(),
        })
    }
}

// ══════════════════════════════════════════════════════════════════
// 容器规格：编排器注入的环境（对照 -e / ConfigMap）
// ══════════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
struct ContainerSpec {
    env: HashMap<String, String>,
    fs: RootFsPolicy,
    shutdown_grace: Duration,
}

impl ContainerSpec {
    fn from_orchestrator_injection() -> Self {
        let mut env = HashMap::new();
        env.insert("KUBERNETES_SERVICE_HOST".into(), "10.96.0.1".into());
        env.insert("POD_NAME".into(), "api-7f8c9".into());
        env.insert("RUST_LOG".into(), "info".into());
        Self {
            env,
            fs: RootFsPolicy::container_default(),
            shutdown_grace: Duration::from_secs(30),
        }
    }

    fn get_env(&self, key: &str) -> Option<&str> {
        self.env.get(key).map(|s| s.as_str())
    }
}

struct MiniRuntime {
    spec: ContainerSpec,
    shutdown: ShutdownCoordinator,
}

impl MiniRuntime {
    fn boot(spec: ContainerSpec) -> Self {
        let grace = spec.shutdown_grace;
        Self {
            spec,
            shutdown: ShutdownCoordinator::new(grace),
        }
    }

    fn handle_signal(&mut self, sig: Signal) -> &str {
        self.shutdown.on_signal(sig)
    }
}

// ══════════════════════════════════════════════════════════════════
// 直观版反模式：无策略写任意路径、无 drain
// ══════════════════════════════════════════════════════════════════

fn naive_write_config() -> Result<(), &'static str> {
    // 陷阱：在镜像内 `/app/config.toml` 写运行时配置 —— 只读层会失败或污染镜像
    let policy = RootFsPolicy {
        read_only_root: true,
        writable_mounts: vec![],
    };
    try_write("/app/config.toml", &policy).map_err(|_| "写入失败仍未处理")
}

fn naive_exit_on_signal() -> ShutdownPhase {
    // 陷阱：收到 SIGTERM 立刻 exit，在途请求被切断
    ShutdownPhase::Exited
}

// ══════════════════════════════════════════════════════════════════
// 进阶版：带 spec 的运行时
// ══════════════════════════════════════════════════════════════════

fn boot_app() -> MiniRuntime {
    MiniRuntime::boot(ContainerSpec::from_orchestrator_injection())
}

fn run_tests() {
    let mut rt = boot_app();
    assert_eq!(
        rt.spec.get_env("POD_NAME"),
        Some("api-7f8c9"),
        "编排器应注入 POD_NAME"
    );

    assert!(try_write("/tmp/cache.dat", &rt.spec.fs).is_ok());
    assert!(
        matches!(
            try_write("/app/config.toml", &rt.spec.fs),
            Err(FsError::ReadOnlyRoot { .. })
        ),
        "只读根应拒绝 /app 写入"
    );

    let msg = rt.handle_signal(Signal::Terminate);
    assert!(msg.contains("drain"));
    assert_eq!(rt.shutdown.phase_label(), "Draining");

    let mut done = false;
    for _ in 0..29 {
        done = rt.shutdown.tick(Duration::from_secs(1));
        if done {
            break;
        }
    }
    assert!(!done, "30s 宽限内不应 prematurely exit");
    assert!(rt.shutdown.tick(Duration::from_secs(1)), "宽限耗尽应 Exited");
    assert_eq!(rt.shutdown.phase_label(), "Exited");

    assert_eq!(naive_exit_on_signal(), ShutdownPhase::Exited);
    assert!(naive_write_config().is_err(), "反模式应暴露失败");

    // 重复 SIGTERM 不应二次 drain
    let mut rt2 = boot_app();
    rt2.handle_signal(Signal::Terminate);
    let again = rt2.handle_signal(Signal::Terminate);
    assert!(again.contains("忽略"));

    let mut rt3 = boot_app();
    let intr = rt3.handle_signal(Signal::Interrupt);
    assert!(intr.contains("drain"));

    println!("✅ l00_container_mental 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ 容器心智：环境 / 只读根 / 信号 ************\n");

    let mut rt = boot_app();
    println!(
        "Pod={} Rust 日志级别={}",
        rt.spec.get_env("POD_NAME").unwrap_or("?"),
        rt.spec.get_env("RUST_LOG").unwrap_or("?"),
    );

    println!("\n--- 文件系统策略 ---");
    for path in ["/tmp/state", "/app/config.toml", "/var/run/health"] {
        match try_write(path, &rt.spec.fs) {
            Ok(()) => println!("允许写入: {path}"),
            Err(e) => println!("{e}"),
        }
    }

    println!("\n--- 收到 SIGTERM（模拟 K8s 删 Pod）---");
    println!("{}", rt.handle_signal(Signal::Terminate));
    let step = Duration::from_secs(5);
    while !rt.shutdown.tick(step) {
        println!(
            "drain 中… 已等待 {:?}",
            rt.shutdown.drained_for
        );
    }
    println!("进程可安全退出（对照 preStop + gracePeriod）");

    println!("\n陷阱回顾:");
    println!("- 勿在只读镜像路径写配置；用 ConfigMap/Secret 挂载或 env");
    println!("- 勿 SIGTERM 后立即 exit；对接 axum graceful shutdown / drain");
    println!("- 本地 `cargo run` 无容器约束 —— 上线前用本心智检查路径与信号");

    run_tests();
}

fn main() {
    bootstrap();
}