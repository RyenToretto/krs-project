/*
[主题]: 生命周期直觉——返回的引用能活多久
[难度]: Medium
[前置]: 01_ownership_borrowing/02_slice

一、主题说明
当你**返回引用**（&str、&[T] 等）时，编译器要知道：
「返回值指向的内存在调用结束后仍然有效吗？」
生命周期标注 `'a` 读作「某段存活期」：告诉编译器多个引用之间的**最小公共存活关系**。
本课只练 **简单函数上的 `'a`**，不讲高阶 Trait 边界（HRTB）。

二、写法对比
1. 直观版: 经典 longest(x, y) -> &'a str。
2. 进阶版: 结构体里带引用字段时也要标 `'a`（本课一个迷你结构体示例）。

三、真实用途
- 解析器返回指向输入缓冲的 &str（零拷贝），必须和输入同寿。
- 缓存键、路由路径片段：引用视图 tied to request buffer。
- 错误信息 `&'static str` 与借自参数的 `&str` 区别（本课点到为止）。
资料: https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html
      https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html
      https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/
*/

/// 直观版：两个 &str 共享同一生命周期 'a，返回值也是这段生命期内有效的引用
fn longest_naive<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() >= y.len() {
        x
    } else {
        y
    }
}

/// 进阶版：只借第一个参数的生命周期（第二个可以是更短的）
fn prefer_first<'a, 'b>(x: &'a str, _y: &'b str) -> &'a str {
    x
}

/// 带引用字段的结构体：标注说明 headline 不能比 article 活得久
struct Excerpt<'a> {
    headline: &'a str,
}

impl<'a> Excerpt<'a> {
    /// 从正文取第一句（到句号或结尾）
    fn first_sentence(article: &'a str) -> Excerpt<'a> {
        let end = article.find('。').map(|i| i + '。'.len_utf8()).unwrap_or(article.len());
        Excerpt {
            headline: &article[..end],
        }
    }
}

/// 返回指向两个切片中较长者（同 'a）
fn longer_slice<'a>(a: &'a [i32], b: &'a [i32]) -> &'a [i32] {
    if a.len() >= b.len() {
        a
    } else {
        b
    }
}

/// 静态字符串：整个程序期间有效（'static，本课作对比）
fn app_name() -> &'static str {
    "krs-learning"
}

fn run_tests() {
    let s1 = String::from("short");
    let s2 = String::from("longer-text");
    // 局部绑定延长到这一行：引用的存活期覆盖 longest 调用
    let picked = longest_naive(s1.as_str(), s2.as_str());
    assert_eq!(picked, "longer-text");

    let a = "alpha";
    let b = "beta-gamma";
    assert_eq!(longest_naive(a, b), b);
    assert_eq!(prefer_first(a, b), a);

    let paper = String::from("Rust 很快。所有权很难。");
    let ex = Excerpt::first_sentence(&paper);
    assert_eq!(ex.headline, "Rust 很快。");

    let left = &[1, 2];
    let right = &[1, 2, 3];
    assert_eq!(longer_slice(left, right), &[1, 2, 3]);

    assert_eq!(app_name(), "krs-learning");

    println!("✅ LifetimeIntro 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 LifetimeIntro ************\n");

    let x = "async";
    let y = "ownership-and-borrowing";
    println!("较长: {}", longest_naive(x, y));

    let article = "生产环境要讲清楚生命周期标注。";
    let excerpt = Excerpt::first_sentence(article);
    println!("摘要: {}", excerpt.headline);

    println!("应用名（'static）: {}", app_name());

    run_tests();
}

fn main() {
    bootstrap();
}