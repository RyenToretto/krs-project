/*
[主题]: 数据库事务心智 —— BEGIN / COMMIT / ROLLBACK（内存模拟）
[难度]: Medium
[前置]: `09_persistence/00_repo`、`09_persistence/01_sqlx`、`06_std_concurrency/02_mutex`（可选）

一、主题说明
- **事务（Transaction）**：多条写操作要么全成功 **提交（COMMIT）**，要么全失败 **回滚（ROLLBACK）**，中间态对外不可见。
- sqlx 典型流程：`let mut tx = pool.begin().await?` → 若干 `query.execute(&mut *tx)` → `tx.commit().await?` 或 `tx.rollback().await?`。
- 本课用 **写时拷贝 + 挂起缓冲** 的 `InMemoryTxStore` 模拟，不连 PostgreSQL。

二、写法对比
1. 直观版: 转账直接改两个 `HashMap` 余额，中途 panic 可能只扣款不加款。
2. 进阶版: 在 `Transaction` 里改「草稿副本」，成功 `commit` 一次性落盘，失败 `rollback` 丢弃草稿。

三、真实用途
- 订单 + 库存扣减、账户转账、发号器 + 插入业务表，常必须在同一事务。
- 隔离级别（READ COMMITTED 等）本课不展开；先建立 commit/rollback 肌肉记忆。
资料: https://docs.rs/sqlx/latest/sqlx/
      https://docs.rs/sqlx/latest/sqlx/trait.Executor.html
      https://github.com/launchbadge/sqlx
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
struct Account {
    id: u64,
    balance_cents: i64,
}

#[derive(Debug, PartialEq)]
enum TxError {
    NotInTransaction,
    AlreadyActive,
    InsufficientFunds { from: u64, need: i64, have: i64 },
    UnknownAccount(u64),
    Internal(String),
}

impl fmt::Display for TxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// 可事务化的账户存储（类比 `Pool` + `Transaction<'_>`）
struct AccountStore {
    committed: HashMap<u64, Account>,
    /// 事务开始时对 committed 的拷贝；None 表示当前无活动事务
    draft: Option<HashMap<u64, Account>>,
}

impl AccountStore {
    fn with_demo_accounts() -> Self {
        let mut committed = HashMap::new();
        committed.insert(
            1,
            Account {
                id: 1,
                balance_cents: 10_000,
            },
        );
        committed.insert(
            2,
            Account {
                id: 2,
                balance_cents: 500,
            },
        );
        Self {
            committed,
            draft: None,
        }
    }

    /// 对应 `pool.begin().await?`
    fn begin(&mut self) -> Result<(), TxError> {
        if self.draft.is_some() {
            return Err(TxError::AlreadyActive);
        }
        self.draft = Some(self.committed.clone());
        Ok(())
    }

    fn in_tx(&self) -> Result<&HashMap<u64, Account>, TxError> {
        self.draft.as_ref().ok_or(TxError::NotInTransaction)
    }

    fn in_tx_mut(&mut self) -> Result<&mut HashMap<u64, Account>, TxError> {
        self.draft.as_mut().ok_or(TxError::NotInTransaction)
    }

    /// 对应 `tx.commit().await?`
    fn commit(&mut self) -> Result<(), TxError> {
        let draft = self.draft.take().ok_or(TxError::NotInTransaction)?;
        self.committed = draft;
        Ok(())
    }

    /// 对应 `tx.rollback().await?`
    fn rollback(&mut self) -> Result<(), TxError> {
        if self.draft.take().is_none() {
            return Err(TxError::NotInTransaction);
        }
        Ok(())
    }

    fn get_committed(&self, id: u64) -> Option<&Account> {
        self.committed.get(&id)
    }
}

/// 在事务草稿里转账（尚未 commit）
fn transfer_in_tx(
    store: &mut AccountStore,
    from: u64,
    to: u64,
    cents: i64,
) -> Result<(), TxError> {
    let book = store.in_tx_mut()?;
    let from_acc = book.get(&from).ok_or(TxError::UnknownAccount(from))?.clone();
    let to_acc = book.get(&to).ok_or(TxError::UnknownAccount(to))?.clone();
    if from_acc.balance_cents < cents {
        return Err(TxError::InsufficientFunds {
            from,
            need: cents,
            have: from_acc.balance_cents,
        });
    }
    book.insert(
        from,
        Account {
            id: from,
            balance_cents: from_acc.balance_cents - cents,
        },
    );
    book.insert(
        to,
        Account {
            id: to,
            balance_cents: to_acc.balance_cents + cents,
        },
    );
    Ok(())
}

// ── 直观版：无事务，两次独立写入 ───────────────────────────────

fn transfer_naive(committed: &mut HashMap<u64, Account>, from: u64, to: u64, cents: i64) -> bool {
    let from_bal = match committed.get(&from) {
        Some(a) => a.balance_cents,
        None => return false,
    };
    if from_bal < cents {
        return false;
    }
    // 只完成第一步就「假装系统崩溃」—— 本函数在真实场景可能 panic 在这行之后
    if let Some(a) = committed.get_mut(&from) {
        a.balance_cents -= cents;
    }
    // 直观版没有 rollback：若下面失败，from 已扣款
    if !committed.contains_key(&to) {
        return false;
    }
    if let Some(a) = committed.get_mut(&to) {
        a.balance_cents += cents;
    }
    true
}

/// 演示直观版在「收款方不存在」时留下脏数据
fn naive_leaves_partial_debit(committed: &mut HashMap<u64, Account>) -> (i64, bool) {
    let _before = committed.get(&1).unwrap().balance_cents;
    let ok = transfer_naive(committed, 1, 999, 100);
    let after = committed.get(&1).unwrap().balance_cents;
    (after, ok)
}

fn run_tests() {
    // 成功事务
    let mut store = AccountStore::with_demo_accounts();
    store.begin().unwrap();
    assert_eq!(store.in_tx().unwrap().len(), 2);
    transfer_in_tx(&mut store, 1, 2, 2000).unwrap();
    store.commit().unwrap();
    assert_eq!(store.get_committed(1).unwrap().balance_cents, 8000);
    assert_eq!(store.get_committed(2).unwrap().balance_cents, 2500);

    // 余额不足 → rollback
    let mut store2 = AccountStore::with_demo_accounts();
    store2.begin().unwrap();
    let err = transfer_in_tx(&mut store2, 2, 1, 999_999);
    assert!(matches!(err, Err(TxError::InsufficientFunds { .. })));
    store2.rollback().unwrap();
    assert_eq!(store2.get_committed(2).unwrap().balance_cents, 500);

    // 不能在无事务时 commit
    let mut store3 = AccountStore::with_demo_accounts();
    assert_eq!(store3.commit(), Err(TxError::NotInTransaction));

    // 直观版脏写
    let mut raw = AccountStore::with_demo_accounts().committed;
    let (after, ok) = naive_leaves_partial_debit(&mut raw);
    assert!(!ok);
    assert_eq!(after, 9900); // 已扣 100，但 to=999 不存在

    assert_eq!(
        format!("{}", TxError::Internal("连接重置".into())),
        "Internal(\"连接重置\")"
    );

    println!("✅ 事务提交/回滚心智（内存）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L02 事务心智 ************\n");

    let mut store = AccountStore::with_demo_accounts();
    println!("初始: acc1={:?}, acc2={:?}", store.get_committed(1), store.get_committed(2));

    store.begin().unwrap();
    match transfer_in_tx(&mut store, 1, 2, 1500) {
        Ok(()) => {
            println!("事务内转账 1500 分成功，准备 commit…");
            store.commit().unwrap();
        }
        Err(e) => {
            println!("事务失败: {} → rollback", e);
            store.rollback().unwrap();
        }
    }
    println!(
        "提交后: acc1={:?}, acc2={:?}",
        store.get_committed(1),
        store.get_committed(2)
    );

    let mut store_fail = AccountStore::with_demo_accounts();
    store_fail.begin().unwrap();
    if transfer_in_tx(&mut store_fail, 2, 1, 10_000).is_err() {
        store_fail.rollback().unwrap();
        println!("余额不足已 rollback，外界仍看到原余额 acc2={:?}", store_fail.get_committed(2));
    }

    println!("\n对照 sqlx: begin → execute on &mut Transaction → commit/rollback");
    println!("文档: https://docs.rs/sqlx/latest/sqlx/\n");

    run_tests();
}

fn main() {
    bootstrap();
}