/*
[主题]: Nomicon 清单 —— 对齐、别名、生命周期不变量（可测自检）
[难度]: Hard
[前置]: 本阶段 `00_unsafe`、`01_ffi`；`01_ownership_borrowing`（别名）

一、主题说明
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) 讲「编译器没全帮你证」的契约；写 unsafe 前应能口头过一遍清单。
- **对齐（alignment）**：类型 `align_of` / `size_of` 决定指针是否可合法解引用为该类型；未对齐时用 `read_unaligned` 仍要证长度与有效内存。
- **别名（aliasing）**：`&mut T` 与 `&T` 的排他规则；`*mut` 可绕过但 **LLVM / UB** 风险在 unsafe 里落在你身上。
- **生命周期不变量**：自管内存的结构要写明「谁分配、谁释放、len/cap 与 ptr 同步」；`PhantomData` 标注牌主。
- 本课用 **纯 safe + 少量对照注释** 实现 checklist 函数，返回 `bool` 供 `run_tests` 断言——模拟 Code Review 勾选项。

二、写法对比
1. 直观版: `assert_eq!(align_of::<u64>(), 8)` 等常量表驱动。
2. 进阶版: `InvariantVec` 在 safe API 里维护 `len <= capacity`，Drop 只通过 `Vec` 释放，不让裸指针泄漏到外部。

三、真实用途
- PR 里审查 unsafe：问对齐、问别名、问 Panic 路径 是否泄漏；Miri / Loom 另练（本课离线自检）。
资料: https://doc.rust-lang.org/nomicon/
      https://doc.rust-lang.org/nomicon/ownership.html
      https://doc.rust-lang.org/nomicon/lifetimes.html
      https://doc.rust-lang.org/reference/
      https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html
*/

use std::marker::PhantomData;
use std::mem::{align_of, size_of};

/// 清单项：名称 + 是否通过
#[derive(Debug, Clone, Copy)]
struct CheckItem {
    name: &'static str,
    ok: bool,
}

// ---------------------------------------------------------------------------
// 1. 对齐与尺寸
// ---------------------------------------------------------------------------

fn check_platform_u64_alignment() -> bool {
    let a = align_of::<u64>();
    a >= 4 && a.is_power_of_two()
}

fn check_size_of_matches_array_len<T, const N: usize>() -> bool {
    size_of::<[T; N]>() == size_of::<T>() * N
}

fn check_ref_has_same_size_as_ptr<T>() -> bool {
    size_of::<&T>() == size_of::<usize>()
}

// ---------------------------------------------------------------------------
// 2. 别名：用 safe Rust 演示「共享借用可同时存在、可变借用排他」
// ---------------------------------------------------------------------------

fn check_shared_refs_can_coexist() -> bool {
    let x = 10u32;
    let r1 = &x;
    let r2 = &x;
    *r1 == *r2 && *r1 == 10
}

fn check_mut_ref_excludes_shared_while_live() -> bool {
    let mut x = 1u32;
    {
        let m = &mut x;
        *m = 2;
    }
    x == 2
}

/// 教学说明：下列函数展示「若同时存在 & 与 &mut 指向同一内存，safe 编译不过」
/// Nomicon 提醒：unsafe 里用裸指针制造别名需要额外契约（本课不演示 UB 写法）。
fn check_safe_alias_rules_documented() -> bool {
    true
}

// ---------------------------------------------------------------------------
// 3. 生命周期不变量：自管「视图」但不外泄裸指针
// ---------------------------------------------------------------------------

struct InvariantVec<T> {
    data: Vec<T>,
    _pin: PhantomData<T>,
}

impl<T> InvariantVec<T> {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            _pin: PhantomData,
        }
    }

    fn push(&mut self, v: T) -> Result<(), &'static str> {
        if self.data.len() == self.data.capacity() && self.data.capacity() > 0 {
            // 仅教学：真实扩容策略不必限制；此处演示「可检测容量不变量」
        }
        self.data.push(v);
        Ok(())
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn get(&self, i: usize) -> Option<&T> {
        self.data.get(i)
    }

    /// 不变量检查：len 永不大于 capacity
    fn check_invariants(&self) -> bool {
        self.data.len() <= self.data.capacity()
    }
}

fn check_invariant_vec_after_ops() -> bool {
    let mut v = InvariantVec::new();
    let _ = v.push(1);
    let _ = v.push(2);
    v.check_invariants() && v.len() == 2 && v.get(1) == Some(&2)
}

// ---------------------------------------------------------------------------
// 4. 汇总清单（供 bootstrap 打印 + run_tests 断言）
// ---------------------------------------------------------------------------

fn run_nomicon_checklist() -> Vec<CheckItem> {
    vec![
        CheckItem {
            name: "u64 对齐为 2 的幂且 >= 4",
            ok: check_platform_u64_alignment(),
        },
        CheckItem {
            name: "[u8;4] 尺寸等于 4 * size_of u8",
            ok: check_size_of_matches_array_len::<u8, 4>(),
        },
        CheckItem {
            name: "&T 与 usize 同大（胖指针除外）",
            ok: check_ref_has_same_size_as_ptr::<u8>(),
        },
        CheckItem {
            name: "共享引用可同时存在",
            ok: check_shared_refs_can_coexist(),
        },
        CheckItem {
            name: "可变借用结束后值已更新",
            ok: check_mut_ref_excludes_shared_while_live(),
        },
        CheckItem {
            name: "safe 别名规则仅在文档课确认（不在此制造 UB）",
            ok: check_safe_alias_rules_documented(),
        },
        CheckItem {
            name: "InvariantVec len/capacity 不变量",
            ok: check_invariant_vec_after_ops(),
        },
    ]
}

fn all_checks_pass(items: &[CheckItem]) -> bool {
    items.iter().all(|c| c.ok)
}

fn run_tests() {
    let items = run_nomicon_checklist();
    for c in &items {
        assert!(c.ok, "Nomicon 清单失败: {}", c.name);
    }
    assert!(all_checks_pass(&items));

    // 对齐相关硬性断言（常见 x86_64 / aarch64 教学环境）
    assert!(align_of::<u64>() <= 8);
    assert_eq!(size_of::<[u8; 4]>(), 4);

    println!("✅ nomicon_checklist 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ Nomicon 可测清单 ************\n");
    let items = run_nomicon_checklist();
    for c in &items {
        let mark = if c.ok { "✓" } else { "✗" };
        println!("  [{mark}] {}", c.name);
    }
    println!(
        "\n说明：写 unsafe 前请对照 Nomicon 章节逐项自问；本清单是离线自检，不替代 Miri。"
    );
    run_tests();
}

fn main() {
    bootstrap();
}