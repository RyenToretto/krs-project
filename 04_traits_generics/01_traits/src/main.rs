/*
[主题]: Trait 定义与 impl —— 行为契约与默认方法
[难度]: Easy → Medium
[前置]: 本阶段 `00_generics`（`impl<T>` 写法）；`02_structs_enums`

一、主题说明
- **Trait** 描述「某类型必须具备哪些能力」，类似其他语言的接口（interface）。
- `impl Trait for Type`：给具体类型实现这套方法；一个类型可以实现多个 trait。
- **默认方法**：trait 里带方法体，类型不重写就用默认实现；可像「钩子」只覆盖一部分。
- **Trait 约束**：`fn foo<T: Summary>(x: T)` 表示只有实现了 `Summary` 的 `T` 才能传进来。

二、写法对比
1. 直观版: 每种类型一个普通函数 `brief_article` / `brief_tweet`，没有统一抽象。
2. 进阶版: 定义 `Summary` trait，`impl Summary for NewsArticle/Tweet`，`notify(item: &impl Summary)`。

三、真实用途
- 标准库：`Debug`、`Clone`、`Iterator` 全是 trait；业务里常自定 `Repository`、`Validator`。
- 可测试性：用 trait 假实现替换数据库，单元测试不连真 DB。
资料: https://doc.rust-lang.org/book/ch10-02-traits.html
      https://doc.rust-lang.org/rust-by-example/trait.html
      https://rust-lang.github.io/api-guidelines/
      https://doc.rust-lang.org/book/
      https://www.youtube.com/@RustVideos
*/

// ── 进阶核心：trait 与默认方法 ──────────────────────────────────

/// 能生成「摘要文本」的类型都实现这个 trait
trait Summary {
    /// 必须实现：作者名
    fn author(&self) -> &str;

    /// 默认方法：子类型不重写就用这段
    fn summarize(&self) -> String {
        format!("{} 发布了内容（默认摘要）", self.author())
    }

    /// 默认方法可调用其它 trait 方法
    fn brief(&self) -> String {
        let s = self.summarize();
        if s.chars().count() > 20 {
            format!("{}…", s.chars().take(20).collect::<String>())
        } else {
            s
        }
    }
}

#[derive(Debug)]
struct NewsArticle {
    title: String,
    author: String,
    /// 正文（本课演示摘要用，测试中不要求读取）
    #[allow(dead_code)]
    body: String,
}

#[derive(Debug)]
struct Tweet {
    username: String,
    content: String,
}

impl Summary for NewsArticle {
    fn author(&self) -> &str {
        &self.author
    }

    fn summarize(&self) -> String {
        format!("《{}》— {}", self.title, self.author)
    }
}

impl Summary for Tweet {
    fn author(&self) -> &str {
        &self.username
    }
    // 不重写 summarize → 使用 trait 默认实现
}

/// 直观版：没有 trait，只能为每种结构体写独立函数
fn brief_article_naive(article: &NewsArticle) -> String {
    format!("文章: {}", article.title)
}

fn brief_tweet_naive(tweet: &Tweet) -> String {
    format!("@{}: {}", tweet.username, tweet.content)
}

/// 进阶：接受「任何实现了 Summary 的引用」—— 静态分发（编译期已知具体类型）
fn notify(item: &impl Summary) {
    println!("[notify] {}", item.summarize());
}

/// 语法糖等价写法：`fn notify<T: Summary>(item: &T)`
fn notify_alt<T: Summary>(item: &T) {
    println!("[notify_alt] {}", item.brief());
}

/// 多个约束：既要能摘要，又要能按 Debug 打印（此处仅演示约束语法）
fn inspect<T: Summary + std::fmt::Debug>(item: &T) {
    println!("inspect Debug={:?} summary={}", item, item.summarize());
}

fn run_tests() {
    let article = NewsArticle {
        title: "Rust 泛型".to_string(),
        author: "Alice".to_string(),
        body: "正文…".to_string(),
    };
    let tweet = Tweet {
        username: "bob_rust".to_string(),
        content: "今天学了 trait".to_string(),
    };

    assert_eq!(brief_article_naive(&article), "文章: Rust 泛型");
    assert!(brief_tweet_naive(&tweet).contains("bob_rust"));

    assert!(article.summarize().contains("Rust 泛型"));
    assert!(tweet.summarize().contains("bob_rust"));
    assert!(tweet.summarize().contains("默认摘要"));

    assert!(article.brief().chars().count() <= 21);
    assert!(tweet.brief().chars().count() <= 25);

    assert_eq!(article.author(), "Alice");
    assert_eq!(tweet.author(), "bob_rust");

    println!("✅ Traits 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L01 Traits ************\n");

    let article = NewsArticle {
        title: "内存安全".to_string(),
        author: "小陈".to_string(),
        body: "……".to_string(),
    };
    let tweet = Tweet {
        username: "learner".to_string(),
        content: "trait 默认方法很省事".to_string(),
    };

    println!("直观: {}", brief_article_naive(&article));
    println!("直观: {}", brief_tweet_naive(&tweet));

    notify(&article);
    notify(&tweet);
    notify_alt(&article);

    inspect(&article);

    run_tests();
}

fn main() {
    bootstrap();
}