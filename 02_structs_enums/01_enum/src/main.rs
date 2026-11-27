/*
[主题]: 枚举（enum）与 match：用类型表达「几种可能」并安全分支
[难度]: Easy
[前置]: 00_rust_basics/01_control、本阶段 00_struct（建议）

一、主题说明
枚举的每个变体（variant）是一种情况；变体可以携带数据（像「带标签的联合体」）。
match 必须覆盖所有变体（穷尽性），编译器帮你防止漏分支。
对比 C/Java：Rust 的 enum + match 是代数数据类型，比整型魔数 + if 更安全。

二、写法对比
1. 直观版: 用多个 bool / 字符串分支 if-else，状态组合容易漏判。
2. 进阶版: enum 建模状态 + match 解构，新增状态必须改 match，编译器会提醒。

三、真实用途
- 订单状态机：Pending / Paid / Shipped / Cancelled。
- HTTP 结果：成功体 vs 错误码 vs 超时，用枚举而不是魔法数字。
资料: https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/book/ch06-00-enums.html
      https://doc.rust-lang.org/rust-by-example/
      https://doc.rust-lang.org/rust-by-example/custom_types/enum.html
*/

/// 简单枚举：只有名字，没有附加数据
#[derive(Debug, PartialEq, Clone, Copy)]
enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

/// 直观版：用数字表示星期，容易传错
fn is_weekend_naive(day_code: i32) -> bool {
    if day_code == 6 || day_code == 7 {
        true
    } else if day_code >= 1 && day_code <= 5 {
        false
    } else {
        false
    }
}

/// 进阶版：用枚举 + match
fn is_weekend_better(day: Weekday) -> bool {
    match day {
        Weekday::Sat | Weekday::Sun => true,
        Weekday::Mon
        | Weekday::Tue
        | Weekday::Wed
        | Weekday::Thu
        | Weekday::Fri => false,
    }
}

/// 带数据的枚举：订单生命周期
#[derive(Debug, PartialEq, Clone)]
enum OrderState {
    /// 刚创建，金额为分
    Pending { amount_cents: i32 },
    Paid { paid_at_day: u32 },
    Shipped { tracking: &'static str },
    Cancelled { reason: &'static str },
}

/// 直观版：用字符串标签 + 手写解析（脆弱）
fn state_label_naive(tag: &str) -> &'static str {
    if tag == "pending" {
        "待支付"
    } else if tag == "paid" {
        "已支付"
    } else if tag == "shipped" {
        "已发货"
    } else if tag == "cancelled" {
        "已取消"
    } else {
        "未知"
    }
}

/// 进阶版：match 同时解构变体里的字段
fn state_label_better(state: &OrderState) -> &'static str {
    match state {
        OrderState::Pending { .. } => "待支付",
        OrderState::Paid { .. } => "已支付",
        OrderState::Shipped { .. } => "已发货",
        OrderState::Cancelled { .. } => "已取消",
    }
}

/// 能否发货：只有 Paid 可以进入 Shipped
fn can_ship(state: &OrderState) -> bool {
    matches!(state, OrderState::Paid { .. })
}

/// 支付：Pending -> Paid；其他状态不变（返回原样克隆）
fn pay(state: OrderState, day: u32) -> OrderState {
    match state {
        OrderState::Pending { amount_cents } if amount_cents > 0 => {
            OrderState::Paid { paid_at_day: day }
        }
        other => other,
    }
}

/// 网络层常见：要么拿到正文，要么拿到错误码
#[derive(Debug, PartialEq)]
enum ApiResult {
    Ok { body: &'static str },
    Err { code: u16, message: &'static str },
}

fn http_summary_naive(code: u16, body: Option<&str>) -> String {
    if code == 200 {
        format!("成功: {}", body.unwrap_or("(空)"))
    } else {
        format!("失败 HTTP {}", code)
    }
}

fn http_summary_better(result: ApiResult) -> String {
    match result {
        ApiResult::Ok { body } => format!("成功: {body}"),
        ApiResult::Err { code, message } => format!("失败 {code}: {message}"),
    }
}

fn run_tests() {
    assert!(!is_weekend_naive(3));
    assert!(is_weekend_naive(7));
    for workday in [
        Weekday::Mon,
        Weekday::Tue,
        Weekday::Wed,
        Weekday::Thu,
        Weekday::Fri,
    ] {
        assert!(!is_weekend_better(workday));
    }
    assert!(is_weekend_better(Weekday::Sat));
    assert!(is_weekend_better(Weekday::Sun));

    let pending = OrderState::Pending { amount_cents: 500 };
    assert_eq!(state_label_naive("pending"), "待支付");
    assert_eq!(state_label_better(&pending), "待支付");
    assert!(!can_ship(&pending));

    let paid = pay(pending, 10);
    assert!(matches!(paid, OrderState::Paid { paid_at_day: 10 }));
    assert!(can_ship(&paid));

    let shipped = OrderState::Shipped {
        tracking: "SF123",
    };
    assert!(!can_ship(&shipped));

    let zero_pending = OrderState::Pending { amount_cents: 0 };
    let still = pay(zero_pending.clone(), 1);
    assert_eq!(still, zero_pending);

    assert_eq!(
        http_summary_naive(200, Some("{\"id\":1}")),
        "成功: {\"id\":1}"
    );
    assert_eq!(
        http_summary_better(ApiResult::Err {
            code: 404,
            message: "not found",
        }),
        "失败 404: not found"
    );

    println!("✅ Enums & match 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L01 Enums & match ************\n");

    println!("周一周末? {} 周六周末? {}", is_weekend_better(Weekday::Mon), is_weekend_better(Weekday::Sat));
    println!(
        "周三是否周末(进阶): {}",
        is_weekend_better(Weekday::Wed)
    );

    let cancelled = OrderState::Cancelled {
        reason: "用户反悔",
    };
    println!("取消单标签: {}", state_label_better(&cancelled));

    let mut order = OrderState::Pending { amount_cents: 3200 };
    println!("状态: {}", state_label_better(&order));
    order = pay(order, 27);
    println!("支付后: {}", state_label_better(&order));
    if can_ship(&order) {
        order = OrderState::Shipped {
            tracking: "YT998877",
        };
        println!("已发货: {:?}", order);
    }

    let api = ApiResult::Ok {
        body: r#"{"status":"ok"}"#,
    };
    println!("API: {}", http_summary_better(api));

    run_tests();
}

fn main() {
    bootstrap();
}