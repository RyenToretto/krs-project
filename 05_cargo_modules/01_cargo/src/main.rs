/*
[主题]: Cargo 心智 + features —— 用条件编译做「可选能力」
[难度]: Easy → Medium
[前置]: `00_modules` 课；会读 `Cargo.toml` 即可

一、主题说明
- **Cargo** 管依赖、构建配置、feature 开关；`[package]`、`[dependencies]`、`[features]` 是日常三件。
- **Feature** 在编译期打开/关闭代码块（`#[cfg(feature = "x")]`），不增加运行时分支开销。
- 默认 feature 写在 `default = [...]`；`cargo build --features demo_extra` 可一次打开多个。

二、写法对比
1. 直观版: `if cfg!(feature = "demo_extra") { ... }` 在运行时走不同 println（心智好记）。
2. 进阶版: `#[cfg(...)]` 让「多出来的函数」在没开 feature 时根本不存在于二进制里。

三、真实用途
- 可选 TLS、可选 metrics：不开 feature 的包更小、编译更快。
- 依赖别人的 crate 时：`some-crate = {{ version = "1", features = ["serde"] }}`。
资料: https://doc.rust-lang.org/cargo/
      https://doc.rust-lang.org/cargo/guide/
      https://doc.rust-lang.org/cargo/reference/features.html
      https://docs.rs/
      https://www.youtube.com/@RustVideos
*/

/// 本课 `Cargo.toml` 声明了 `demo_extra` feature（见同目录清单文件）

/// 直观版：运行时用 `cfg!` 宏读 **编译期** 是否启用了 feature（结果是 `bool` 常量）
fn build_banner_naive() -> String {
    let mut parts = vec!["krs-project".to_string(), "edition=2021".to_string()];
    if cfg!(feature = "demo_extra") {
        parts.push("extra=on".to_string());
    } else {
        parts.push("extra=off".to_string());
    }
    parts.join(" | ")
}

/// 进阶版：只有启用 `demo_extra` 时，编译器才会包含此函数
#[cfg(feature = "demo_extra")]
fn extra_capability() -> &'static str {
    "metrics_stub_enabled"
}

/// 没开 feature 时的占位实现，保证调用方签名一致
#[cfg(not(feature = "demo_extra"))]
fn extra_capability() -> &'static str {
    "metrics_stub_disabled"
}

#[cfg(feature = "demo_extra")]
fn describe_profile_better() -> &'static str {
    "profile: full (demo_extra)"
}

#[cfg(not(feature = "demo_extra"))]
fn describe_profile_better() -> &'static str {
    "profile: lean (default features)"
}

/// 解析「像在 Cargo.toml 里写给依赖开 feature」的一行（教学用迷你解析）
fn parse_dep_feature_line_naive(line: &str) -> Option<(String, Vec<String>)> {
    let line = line.trim();
    if !line.contains("features") {
        return None;
    }
    // 期望形如: serde = { version = "1", features = ["derive"] }
    let name = line.split('=').next()?.trim().to_string();
    let mut feats = Vec::new();
    if let Some(start) = line.find('[') {
        if let Some(end) = line.find(']') {
            let inside = &line[start + 1..end];
            for part in inside.split(',') {
                let f = part.trim().trim_matches('"');
                if !f.is_empty() {
                    feats.push(f.to_string());
                }
            }
        }
    }
    if feats.is_empty() {
        None
    } else {
        Some((name, feats))
    }
}

fn manifest_mental_model() -> &'static str {
    r#"
[package]
name = "my_api"
version = "0.1.0"
edition = "2021"

[features]
default = []
https = ["dep:rustls"]   # 可选 TLS

[dependencies]
axum = "0.8"
serde = { version = "1", features = ["derive"] }
"#
}

fn run_tests() {
    let banner = build_banner_naive();
    assert!(banner.contains("krs-project"));
    #[cfg(feature = "demo_extra")]
    assert!(banner.contains("extra=on"));
    #[cfg(not(feature = "demo_extra"))]
    assert!(banner.contains("extra=off"));

    assert_eq!(extra_capability(), {
        #[cfg(feature = "demo_extra")]
        {
            "metrics_stub_enabled"
        }
        #[cfg(not(feature = "demo_extra"))]
        {
            "metrics_stub_disabled"
        }
    });

    let parsed = parse_dep_feature_line_naive(r#"serde = { version = "1", features = ["derive"] }"#)
        .expect("应能解析示例行");
    assert_eq!(parsed.0, "serde");
    assert_eq!(parsed.1, vec!["derive".to_string()]);

    assert!(manifest_mental_model().contains("[features]"));
    assert!(manifest_mental_model().contains("serde"));

    println!("✅ Cargo 与 features 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Cargo 与 features ************\n");

    println!("构建条幅（直观）: {}", build_banner_naive());
    println!("能力位（进阶 cfg）: {}", extra_capability());
    println!("描述: {}", describe_profile_better());

    println!("\n尝试本课可选 feature:");
    println!("  cargo run -p l00_cargo_features --features demo_extra");
    println!("  （在 verify 默认跑法下通常为 default，无 demo_extra）");

    if let Some((crate_name, feats)) =
        parse_dep_feature_line_naive(r#"serde = { version = "1", features = ["derive"] }"#)
    {
        println!("依赖 feature 解析示例: {crate_name} -> {:?}", feats);
    }

    println!("\n--- Cargo.toml 心智片段（字符串示意）---");
    println!("{}", manifest_mental_model());

    run_tests();
}

fn main() {
    bootstrap();
}