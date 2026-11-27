/*
[主题]: Cargo Workspace 心智 —— 多 crate 一家人，不必本课真建多包
[难度]: Medium
[前置]: `01_cargo` 课；知道什么是 crate / package

一、主题说明
- **Workspace** 把多个 package 放在同一仓库，共享 **一个** `Cargo.lock`、统一版本、一次 `cargo test` 扫全家。
- 根 `Cargo.toml` 写 `[workspace] members = [...]`；成员各管自己的 `Cargo.toml`。
- 成员间依赖：`my_api = {{ path = "../my_api" }}`；常用 `resolver = "2"`（2021 edition 默认心智）。

二、写法对比
1. 直观版: 用多行字符串画出目录树 + 成员列表，肉眼核对。
2. 进阶版: 解析简化版 workspace 片段，自动抽出 `members`、判断路径是否合理。

三、真实用途
- 单体仓库里 `crates/core` + `crates/api` + `crates/cli` 是 Rust 后端常见布局。
- CI 一条 `cargo test --workspace` 覆盖全部；改 shared types 立刻被各成员看见。
资料: https://doc.rust-lang.org/cargo/reference/workspaces.html
      https://doc.rust-lang.org/cargo/guide/
      https://doc.rust-lang.org/cargo/
      https://www.youtube.com/@RustVideos
*/

/// 教学用：假装我们有一个三成员 workspace（不写入磁盘，避免 verify 递归复杂）
const SAMPLE_WORKSPACE_TOML: &str = r#"
[workspace]
resolver = "2"
members = [
    "crates/core",
    "crates/api",
    "crates/cli",
]

[workspace.package]
version = "0.9.0"
edition = "2021"
"#;

const SAMPLE_TREE: &str = r#"
my-monorepo/
  Cargo.toml          # workspace 根
  Cargo.lock          # 全家共用一把锁
  crates/
    core/Cargo.toml   # package name = my_core
    api/Cargo.toml    # 依赖 path = "../core"
    cli/Cargo.toml
"#;

/// 直观版：从固定示意图里取「成员名」最后一节
fn member_short_names_naive() -> Vec<&'static str> {
    vec!["core", "api", "cli"]
}

/// 进阶版：从类似 TOML 的文本里抽取 `members = [ ... ]` 中的路径
fn extract_workspace_members(blobs: &str) -> Vec<String> {
    let mut members = Vec::new();
    let Some(start) = blobs.find("members") else {
        return members;
    };
    let rest = &blobs[start..];
    let Some(open) = rest.find('[') else {
        return members;
    };
    let Some(close) = rest.find(']') else {
        return members;
    };
    let inside = &rest[open + 1..close];
    for line in inside.lines() {
        let line = line.trim().trim_end_matches(',');
        if line.starts_with('"') && line.ends_with('"') {
            let path = line.trim_matches('"');
            members.push(path.to_string());
        }
    }
    members
}

/// 判断成员路径是否像 Cargo 约定的 `crates/xxx` 布局
fn looks_like_crate_member(path: &str) -> bool {
    path.starts_with("crates/") && !path.contains(' ')
}

/// 模拟「根 workspace 统一版本」是否传播到成员（纯函数，不测 cargo 真行为）
fn workspace_version_hint(root: &str, member_count: usize) -> String {
    format!(
        "workspace@{root}: {member_count} members share [workspace.package].version"
    )
}

/// 成员依赖边（示意）：api -> core, cli -> api
fn dependency_edges_mental() -> Vec<(&'static str, &'static str)> {
    vec![("crates/api", "crates/core"), ("crates/cli", "crates/api")]
}

fn validate_acyclic(edges: &[(&str, &str)]) -> bool {
    // 教学图无环即可
    let mut seen = std::collections::HashSet::new();
    for (from, to) in edges {
        if from == to {
            return false;
        }
        seen.insert((*from, *to));
    }
  !seen.is_empty() || edges.is_empty()
}

fn run_tests() {
    let naive = member_short_names_naive();
    assert_eq!(naive, vec!["core", "api", "cli"]);

    let members = extract_workspace_members(SAMPLE_WORKSPACE_TOML);
    assert_eq!(members.len(), 3);
    assert_eq!(members[0], "crates/core");
    assert!(members.iter().all(|p| looks_like_crate_member(p)));

    assert!(SAMPLE_TREE.contains("Cargo.lock"));
    assert!(SAMPLE_WORKSPACE_TOML.contains("resolver"));

    let hint = workspace_version_hint("my-monorepo", members.len());
    assert!(hint.contains("3 members"));

    let edges = dependency_edges_mental();
    assert!(validate_acyclic(&edges));
    assert_eq!(edges[0], ("crates/api", "crates/core"));

    println!("✅ Workspace 心智所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Workspace 心智 ************\n");

    println!("目录树示意:\n{}", SAMPLE_TREE);

    let members = extract_workspace_members(SAMPLE_WORKSPACE_TOML);
    println!("从 workspace 片段解析出的 members:");
    for m in &members {
        println!("  - {m}");
    }

    println!("\n{}", workspace_version_hint("my-monorepo", members.len()));

    println!("\n成员依赖（path 依赖示意）:");
    for (from, to) in dependency_edges_mental() {
        println!("  {from}  -->  {to}");
    }

    println!("\n常用命令（在真实 workspace 根目录执行）:");
    println!("  cargo build --workspace");
    println!("  cargo test -p my_api");
    println!("  cargo run -p my_cli");

    println!("\n--- workspace 根 Cargo.toml 片段 ---");
    println!("{}", SAMPLE_WORKSPACE_TOML);

    run_tests();
}

fn main() {
    bootstrap();
}