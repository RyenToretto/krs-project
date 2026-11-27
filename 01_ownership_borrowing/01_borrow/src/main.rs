/*
[主题]: 借用（Borrowing）——不拿走所有权也能用数据
[难度]: Easy
[前置]: 01_ownership_borrowing/00_move

一、主题说明
- &T：**不可变借用**，可以同时存在多个，大家都只能读。
- &mut T：**可变借用**，同一时刻最多一个，且不能与不可变借用重叠。
违反规则时编译器直接拒绝（避免数据竞争）。借用有「作用域」，出了作用域就还回去。

二、写法对比
1. 直观版: 把 &Vec 传进去只读求和；需要改数据就传 &mut。
2. 进阶版: 参数写成切片 &[i32]（下一课），比 &Vec 更通用。

三、真实用途
- 多线程前：借用规则是编译期「读-写锁」的雏形。
- 解析 HTTP 头、遍历配置表：大量 &str / &[T] 只读借用。
- 原地更新计数器、缓冲写入：&mut。
资料: https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html
      https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html
      https://doc.rust-lang.org/book/
      https://doc.rust-lang.org/rust-by-example/scope/borrow.html
*/

/// 直观版：不可变借用，遍历求和
fn sum_naive(nums: &Vec<i32>) -> i32 {
    let mut total = 0;
    for n in nums {
        total += *n; // 解引用只读地读每个 i32
    }
    total
}

/// 进阶版：接受切片，Vec 数组字面量都能传（本课先用 &Vec，逻辑相同）
fn sum_better(nums: &[i32]) -> i32 {
    nums.iter().sum()
}

/// 直观版：可变借用，把每个元素加 delta
fn bump_naive(nums: &mut Vec<i32>, delta: i32) {
    for n in nums.iter_mut() {
        *n += delta;
    }
}

/// 进阶版：只改第一个元素（常见「改状态」入口）
fn bump_head(nums: &mut [i32], delta: i32) {
    if let Some(first) = nums.first_mut() {
        *first += delta;
    }
}

/// 演示：先不可变借用读，作用域结束后再可变借用写
fn read_then_write(nums: &mut Vec<i32>) -> (i32, i32) {
    let sum_before = sum_better(nums); // 不可变借用
    bump_naive(nums, 1); // 可变借用（上一轮借用已结束）
    let sum_after = sum_better(nums);
    (sum_before, sum_after)
}

/// 取不可变借用返回其中一项的拷贝（避免返回引用带来的生命周期困惑，本课先练借用）
fn max_copy(nums: &[i32]) -> Option<i32> {
    nums.iter().copied().max()
}

fn run_tests() {
    let data = vec![1, 2, 3];
    assert_eq!(sum_naive(&data), 6);
    assert_eq!(sum_better(&data), 6);
    assert_eq!(sum_better(&[10, 20]), 30); // 切片字面量

    let mut mutable = vec![5, 10];
    bump_naive(&mut mutable, 2);
    assert_eq!(mutable, vec![7, 12]);

    let mut head = vec![100, 1, 1];
    bump_head(&mut head, 50);
    assert_eq!(head[0], 150);

    let (before, after) = read_then_write(&mut vec![0, 1]);
    assert_eq!(before, 1);
    assert_eq!(after, 3);

    assert_eq!(max_copy(&[3, 9, 4]), Some(9));
    assert_eq!(max_copy(&[]), None);

    println!("✅ Borrowing 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L00 Borrowing ************\n");

    let scores = vec![80, 90, 100];
    println!("总分（不可变借用）: {}", sum_better(&scores));

    let mut cfg = vec![1, 2, 3];
    bump_naive(&mut cfg, 10);
    println!("加 10 后: {:?}", cfg);

    run_tests();
}

fn main() {
    bootstrap();
}