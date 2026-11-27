/*
[主题]: `dyn Trait` 与静态分发 —— 何时用「接口指针」
[难度]: Medium
[前置]: 本阶段 `01_traits`；理解 `&impl Trait` 与泛型约束

一、主题说明
- **静态分发**（泛型 / `impl Trait`）：编译器为每个具体类型生成专门函数，性能接近 C++ 模板，体积可能变大。
- **动态分发**（`&dyn Trait` / `Box<dyn Trait>`）：运行时通过**虚表（vtable）**调到正确实现；类型在运行时才知道。
- 同一变量**不能**同时是两种不同的具体类型，除非用 `dyn Trait` 把不同类型「擦成」同一接口（常配合 `Box`）。
- 原则：热路径、类型固定 → 泛型；要放进 `Vec` 里混多种实现 → `Box<dyn Trait>`。

二、写法对比
1. 直观版: 两个函数 `greet_dog` / `greet_cat`，没法把猫狗放进同一个 `Vec` 统一处理。
2. 进阶版: `trait Animal`，`greet_static<T: Animal>` vs `greet_dyn(&dyn Animal)`，`Vec<Box<dyn Animal>>`。

三、真实用途
- 插件、策略模式：运行时选不同实现（日志输出到文件 / 控制台）。
- 异步里 `Pin<Box<dyn Future<Output = T>>>` 是进阶话题；本课只建立「dyn = 运行时多态」直觉。
资料: https://doc.rust-lang.org/book/ch10-02-traits.html
      https://doc.rust-lang.org/book/ch17-02-trait-objects.html
      https://doc.rust-lang.org/rust-by-example/trait.html
      https://doc.rust-lang.org/book/
      https://www.youtube.com/@RustVideos
*/

trait Animal {
    fn name(&self) -> &str;
    fn speak(&self) -> String;
}

struct Dog {
    nickname: String,
}

struct Cat {
    nickname: String,
}

impl Animal for Dog {
    fn name(&self) -> &str {
        &self.nickname
    }

    fn speak(&self) -> String {
        format!("{}: 汪汪", self.nickname)
    }
}

impl Animal for Cat {
    fn name(&self) -> &str {
        &self.nickname
    }

    fn speak(&self) -> String {
        format!("{}: 喵喵", self.nickname)
    }
}

// ── 直观版：每种动物单独函数，无法统一容器 ──────────────────────

fn greet_dog_naive(d: &Dog) -> String {
    format!("你好 {}", d.nickname)
}

fn greet_cat_naive(c: &Cat) -> String {
    format!("你好 {}", c.nickname)
}

// ── 静态分发：编译期单态化，适合类型在编写时就确定 ───────────────

fn greet_static<T: Animal>(pet: &T) -> String {
    format!("【静态】{} 说: {}", pet.name(), pet.speak())
}

/// `impl Animal` 作为参数类型，也是静态分发（语法糖）
fn speak_impl(pet: &impl Animal) -> String {
    pet.speak()
}

// ── 动态分发：trait 对象，适合「同一张表里多种实现」 ─────────────

fn greet_dyn(pet: &dyn Animal) -> String {
    format!("【动态】{} 说: {}", pet.name(), pet.speak())
}

/// 收集不同具体类型，统一按 Animal 处理
fn zoo_lineup(pets: &[Box<dyn Animal>]) -> Vec<String> {
    pets.iter().map(|p| greet_dyn(p.as_ref())).collect()
}

fn run_tests() {
    let dog = Dog {
        nickname: "Lucky".to_string(),
    };
    let cat = Cat {
        nickname: "Mimi".to_string(),
    };

    assert!(greet_dog_naive(&dog).contains("Lucky"));
    assert!(greet_cat_naive(&cat).contains("Mimi"));

    assert!(greet_static(&dog).contains("静态"));
    assert!(greet_static(&cat).contains("喵喵"));
    assert!(speak_impl(&dog).contains("汪汪"));

    let dog_dyn: &dyn Animal = &dog;
    let cat_dyn: &dyn Animal = &cat;
    assert!(greet_dyn(dog_dyn).contains("动态"));
    assert!(greet_dyn(cat_dyn).contains("Mimi"));

    let pets: Vec<Box<dyn Animal>> = vec![Box::new(dog), Box::new(cat)];
    let lines = zoo_lineup(&pets);
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("汪汪") || lines[1].contains("汪汪"));
    assert!(lines[0].contains("喵喵") || lines[1].contains("喵喵"));

    println!("✅ dyn Trait 所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ L02 dyn Trait ************\n");

    let dog = Dog {
        nickname: "阿黄".to_string(),
    };
    let cat = Cat {
        nickname: "小花".to_string(),
    };

    println!("{}", greet_static(&dog));
    println!("{}", greet_dyn(&cat));

    let pets: Vec<Box<dyn Animal>> = vec![Box::new(dog), Box::new(cat)];
    for line in zoo_lineup(&pets) {
        println!("{line}");
    }

    println!("\n提示: `greet_static` 在编译时选定具体类型；`greet_dyn` 通过 vtable 在运行时查找实现。");

    run_tests();
}

fn main() {
    bootstrap();
}