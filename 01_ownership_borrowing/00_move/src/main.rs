/*
[主题]: 所有权与 move——谁「拥有」这份数据
[难度]: Easy
[前置]: 00_rust_basics（类型、String 入门）

一、主题说明
Rust 里，每个值在同一时刻只属于一个变量（一个 owner）。
把值赋给新变量、或传给要「拿走数据」的函数时，**所有权会移动（move）**，
旧变量就不能再用了（编译器会报错，帮你避免重复释放内存）。
整数等简单类型实现了 Copy，赋值时是**拷贝**而不是 move，所以两边都还能用。

二、写法对比
1. 直观版: 函数参数写 String，调用方交出所有权；需要再用就 clone。
2. 进阶版: 只读场景用 &str / 借用（下一课细讲），减少不必要的 move 和 clone。

三、真实用途
- HTTP 处理器里把 Body 字符串交给业务层：要么 move 进去，要么传引用。
- 把配置 String 放进结构体字段时，常发生 move。
- 不必要的 clone 是性能热点，先想清楚谁该拥有数据。
资料: https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html
      https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html
      https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/scope/move.html
*/

/// 简单类型：i32 实现了 Copy，赋值后 a、b 都有效
fn demo_copy() -> (i32, i32) {
    let a = 10; // 绑定一个整数
    let b = a; // 拷贝一份，不是 move
    (a, b) // 两个都能用
}

/// 直观版：接收 String，调用方之后不能再用它（除非先 clone）
fn stringify_len_naive(payload: String) -> usize {
  payload.len()
}

/// 进阶版：只读长度用 &str，不拿走堆上的 String（caller 仍拥有原 String）
fn stringify_len_better(payload: &str) -> usize {
    payload.len()
}

/// 直观版：把两个 String 拼成一个新 String（旧的两段可被 drop）
fn join_naive(mut left: String, right: String) -> String {
    left.push_str(&right); // right 被借用拼接，随后 join 的结果拥有新缓冲区
    left // move 出新 String 给调用方
}

/// 进阶版：用 format! 表达意图更清晰（仍会分配新 String）
fn join_better(left: &str, right: &str) -> String {
    format!("{left}{right}")
}

/// 演示：即将 move 前先 clone，原数据在调用后仍可用
fn demonstrate_move_needs_clone() -> (usize, usize) {
    let original = String::from("rust");
    let backup = original.clone(); // 先拷贝一份给自己留着
    let n = stringify_len_naive(original); // original 被 move 走
    let n2 = stringify_len_better(&backup); // 还能用 backup
    (n, n2)
}

fn owner_label(who: String) -> String {
    format!("owner={who}")
}

fn run_tests() {
    let (x, y) = demo_copy();
    assert_eq!(x, 10);
    assert_eq!(y, 10);

    let msg = String::from("hello");
    let len1 = stringify_len_better(&msg); // 只借看一眼，msg 仍有效
    assert_eq!(len1, 5);
    assert_eq!(msg, "hello");

    let moved_msg = String::from("world");
    let len2 = stringify_len_naive(moved_msg); // move 进去
    assert_eq!(len2, 5);
    // moved_msg 此处已不可用——编译器若在下面写 moved_msg 会报错

    let a = String::from("foo");
    let b = String::from("bar");
    let joined = join_better(&a, &b);
    assert_eq!(joined, "foobar");
    assert_eq!(a, "foo"); // 没 move，a、b 还在

    let j = join_naive(String::from("a"), String::from("b"));
    assert_eq!(j, "ab");

    let (n1, n2) = demonstrate_move_needs_clone();
    assert_eq!(n1, 4);
    assert_eq!(n2, 4);

    let tag = owner_label(String::from("svc-a"));
    assert_eq!(tag, "owner=svc-a");

    println!("✅ OwnershipMove 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 OwnershipMove ************\n");

    let (a, b) = demo_copy();
    println!("Copy 示例: a={a}, b={b}");

    let name = String::from("Rustacean");
    println!("长度（借用）: {}", stringify_len_better(&name));
    println!("仍可使用 name: {name}");

    let svc = join_better("api-", "gateway");
    println!("拼接: {svc}");

    run_tests();
}

fn main() {
    bootstrap();
}