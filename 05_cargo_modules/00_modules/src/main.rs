/*
[主题]: 模块（mod）与可见性（pub）—— 在一个文件里搭出「小 crate」
[难度]: Easy → Medium
[前置]: `00_rust_basics`、`02_structs_enums`；`04_traits_generics` 建议已学

一、主题说明
- Rust 用 **模块** 组织代码：crate 根是 `main.rs` 或 `lib.rs`，子模块用 `mod` 声明。
- **默认私有**：同模块内可见；`pub` 才对外；`pub(crate)` 仅当前 crate；`pub(super)` 仅父模块。
- `use` 缩短路径；`pub use` **再导出**，把内部实现藏起来、对外只暴露稳定 API（库 crate 日常）。

二、写法对比
1. 直观版: 所有函数堆在根模块，靠长名字区分「对外 / 内部」。
2. 进阶版: 子模块分区 + `pub use` 门面，调用方只见 `app::health_check()`。

三、真实用途
- 后端：按 `routes` / `domain` / `infra` 分 mod，测试只测 `pub` API。
- 二进制里也经常把逻辑拆进 `lib.rs` + 多个 `mod` 再在 `main` 里 `use`。
资料: https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
      https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html
      https://doc.rust-lang.org/book/
      https://www.youtube.com/@RustVideos
*/

// ── 直观版：扁平结构，用名字前缀暗示「别在外部用」 ──────────────

fn public_greeting_naive(name: &str) -> String {
    format!("你好，{}", name.trim())
}

fn _internal_audit_naive(action: &str) -> String {
    // 下划线开头是约定俗成的「内部」；编译器不禁止别人调用，但 Clippy 会提醒你。
    format!("[audit] {action}")
}

fn register_user_naive(name: &str) -> String {
    let msg = public_greeting_naive(name);
    let _ = _internal_audit_naive("register");
    msg
}

// ── 进阶版：模块树 + 可见性关键字 ───────────────────────────

mod app {
    /// 对外 HTTP 相关（真实项目里常再拆文件 `routes.rs`）
    pub mod http {
        /// 完全公开：其他 crate 也能用（若本包是 lib）
        pub fn health() -> &'static str {
            "ok"
        }

        /// 私有：只有 `http` 模块内可调用
        fn log_route(path: &str) {
            let _ = path; // 本课不打印，避免 bootstrap 太吵
        }

        pub fn describe_get_health() -> String {
            log_route("/health");
            format!("GET /health -> {}", health())
        }
    }

    /// 仅当前 crate 可见：同项目的集成测试能访问，外部依赖者不能
    pub(crate) fn crate_only_marker() -> &'static str {
        "krs-demo-crate"
    }

    mod domain {
        /// 父模块 `app` 及其子模块可访问（`super` = 上一级 mod）
        pub(super) fn validate_name(name: &str) -> Result<&str, &'static str> {
            let name = name.trim();
            if name.is_empty() {
                Err("名字不能为空")
            } else {
                Ok(name)
            }
        }
    }

    /// 对外注册 API：内部校验藏在 `domain` 里
    pub fn register(name: &str) -> Result<String, &'static str> {
        let clean = domain::validate_name(name)?;
        Ok(format!("欢迎，{clean}"))
    }

    /// 再导出：调用方写 `app::health_check()` 而不必知道在 `http` 里
    pub use http::health as health_check;
}

/// 门面风格：根模块再导出，二进制 main 只 `use` 少量名字
pub use app::health_check;

fn run_tests() {
    assert_eq!(
        register_user_naive(" 阿花 "),
        "你好，阿花"
    );
    assert_eq!(public_greeting_naive("Bob"), "你好，Bob");

    assert_eq!(app::http::health(), "ok");
    assert_eq!(
        app::http::describe_get_health(),
        "GET /health -> ok"
    );
    assert_eq!(app::crate_only_marker(), "krs-demo-crate");
    assert_eq!(app::register("小李").unwrap(), "欢迎，小李");
    assert!(app::register("   ").is_err());
    assert_eq!(health_check(), "ok");

    // 心智：若在别的模块写 `app::http::log_route` 或 `app::domain::...` 会 E0603（私有）

    println!("✅ 模块与可见性所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 模块与可见性 ************\n");

    println!("直观版: {}", register_user_naive("新手"));
    println!("进阶门面 health: {}", health_check());
    println!("进阶路由说明: {}", app::http::describe_get_health());
    match app::register("模块课学员") {
        Ok(s) => println!("注册: {s}"),
        Err(e) => println!("注册失败: {e}"),
    }

    println!("\n可见性速记:");
    println!("  无 pub     → 当前 mod 及子 mod");
    println!("  pub(crate) → 整个 crate");
    println!("  pub(super) → 父 mod");
    println!("  pub        → 父链一路公开");

    run_tests();
}

fn main() {
    bootstrap();
}