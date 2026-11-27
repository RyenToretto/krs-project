# krs-project 阶段路线

## 学习者画像

- **起点**：Rust 小白（可有其他语言基础，但按零基础讲 Rust）。
- **目标**：先达到**可就业 Rust 后端/系统向**，再沿 `16→17→18→19→20→21` 冲刺各方向**顶尖精通**。
- **工具链**：rustup 最新稳定 `rustc`/`cargo`。
- **资料**：每知识点资料包见 [resources.md](resources.md)。

## 阶段编号（学习顺序）

| 编号 | 目录 | 内容 |
| --- | --- | --- |
| `00_` | `00_rust_basics` | 安装、cargo、类型、控制流、函数 |
| `01_` | `01_ownership_borrowing` | 所有权、借用、引用、切片入门 |
| `02_` | `02_structs_enums` | 结构体、枚举、match、Option |
| `03_` | `03_error_collections` | Result、Vec、HashMap、String |
| `04_` | `04_traits_generics` | 泛型、trait、dyn Trait |
| `05_` | `05_cargo_modules` | 模块、crate、workspace、features |
| `06_` | `06_std_concurrency` | thread、channel、Mutex、Arc |
| `07_` | `07_async_tokio` | async/await、Tokio |
| `08_` | `08_web_axum` | Axum / Tower 心智 |
| `09_` | `09_persistence` | sqlx / 持久化心智 |
| `10_` | `10_caching_redis` | 缓存与 Redis 客户端心智 |
| `11_` | `11_messaging` | 异步解耦、至少一次与幂等 |
| `12_` | `12_grpc_tonic` | 拆分、tonic/gRPC 心智 |
| `13_` | `13_testing` | 单元测试、集成测试、基准 |
| `14_` | `14_observability` | tracing、指标、追踪 |
| `15_` | `15_project_capstone` | 鉴权+CRUD+缓存综合 |
| `16_` | `16_production_rust` | 错误模型、配置、弹性、Clippy |
| `17_` | `17_async_advanced` | Pin、Future、取消与陷阱 |
| `18_` | `18_unsafe_ffi` | unsafe、FFI、Nomicon 心智 |
| `19_` | `19_perf_memory` | 性能、剖析、分配器直觉 |
| `20_` | `20_cloud_native` | 容器、健康检查、配置密钥 |
| `21_` | `21_architecture` | 架构、ADR、SRE |

精通推荐学习顺序：`16 → 17 → 18 → 19 → 20 → 21`。

## 课表明细（生成时按此落盘；可增不可随意删核心课）

### `00_rust_basics`（4）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_hello` | `l00_hello_and_types` | main、基本类型、自测结构 |
| `01_control` | `l00_control_flow` | if / match / for / loop |
| `02_functions` | `l00_functions_and_modules_intro` | 函数、参数、返回值 |
| `03_strings` | `l00_strings_and_slices_intro` | String / &str 入门 |

### `01_ownership_borrowing`（4）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_move` | `l00_ownership_move` | 所有权与 move |
| `01_borrow` | `l00_borrowing` | 不可变/可变借用 |
| `02_slice` | `l00_slices` | 切片 |
| `03_lifetime_intro` | `l00_lifetime_intro` | 生命周期直觉 |

### `02_structs_enums`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_struct` | `l00_structs` | 结构体与方法 |
| `01_enum` | `l00_enums_match` | 枚举与 match |
| `02_option` | `l00_option_patterns` | Option 惯用法 |

### `03_error_collections`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_result` | `l00_result_error` | Result / ? |
| `01_vec_map` | `l00_vec_hashmap` | Vec / HashMap |
| `02_string` | `l00_string_deep` | String 进阶 |

### `04_traits_generics`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_generics` | `l00_generics` | 泛型 |
| `01_traits` | `l00_traits` | trait |
| `02_dyn` | `l00_dyn_trait` | dyn Trait |

### `05_cargo_modules`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_modules` | `l00_modules_visibility` | 模块与可见性 |
| `01_cargo` | `l00_cargo_features` | Cargo / features |
| `02_workspace` | `l00_workspace_mental` | workspace 心智 |

### `06_std_concurrency`（4）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_thread` | `l00_threads` | thread |
| `01_channel` | `l00_channels` | mpsc |
| `02_mutex` | `l00_mutex_arc` | Mutex / Arc |
| `03_pitfalls` | `l00_concurrency_pitfalls` | 死锁等陷阱 |

### `07_async_tokio`（4）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_async` | `l00_async_await` | async/await |
| `01_runtime` | `l00_tokio_runtime` | Tokio runtime |
| `02_select` | `l00_tokio_select` | select! / join |
| `03_pitfalls` | `l00_async_pitfalls` | 阻塞陷阱 |

### `08_web_axum`（4）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_router` | `l00_axum_router_mental` | Router 心智 |
| `01_extractor` | `l00_extractors_mental` | Extractor |
| `02_middleware` | `l00_tower_middleware_mental` | Tower 中间件 |
| `03_layers` | `l00_handler_service_repo` | 分层 |

### `09_persistence`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_repo` | `l00_repo_trait` | Repository 心智 |
| `01_sqlx` | `l00_sqlx_mental` | sqlx 心智 |
| `02_tx` | `l00_transactions_mental` | 事务 |

### `10_caching_redis`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_cache` | `l00_cache_aside` | Cache-Aside |
| `01_client` | `l00_redis_client_mental` | 客户端心智 |
| `02_lock` | `l00_distributed_lock_intro` | 锁入门 |

### `11_messaging`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_async` | `l00_async_decouple` | 异步解耦 |
| `01_idempotent` | `l00_at_least_once_idempotent` | 幂等 |
| `02_broker` | `l00_broker_mental` | Broker 心智 |

### `12_grpc_tonic`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_split` | `l00_service_split` | 拆分 |
| `01_tonic` | `l00_tonic_mental` | tonic/gRPC |
| `02_gateway` | `l00_gateway_mental` | 网关 |

### `13_testing`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_unit` | `l00_unit_table_driven` | 表驱动 |
| `01_mock` | `l00_mock_trait` | trait mock |
| `02_integration` | `l00_integration_test_mental` | 集成测试心智 |

### `14_observability`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_tracing` | `l00_tracing_mental` | tracing |
| `01_metrics` | `l00_metrics_mental` | 指标 |
| `02_otel` | `l00_otel_mental` | OTel 心智 |

### `15_project_capstone`（1）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_app` | `l00_mini_api_capstone` | 鉴权+CRUD+缓存 |

### `16_production_rust`（4）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_errors` | `l00_thiserror_anyhow` | 错误模型 |
| `01_config` | `l00_config_mental` | 配置 |
| `02_resilience` | `l00_resilience` | 弹性 |
| `03_clippy` | `l00_clippy_api_guidelines` | Clippy / 指南 |

### `17_async_advanced`（4）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_future` | `l00_future_intuition` | Future 心智 |
| `01_pin` | `l00_pin_mental` | Pin |
| `02_cancel` | `l00_cancellation` | 取消 |
| `03_traps` | `l00_async_traps` | 进阶陷阱 |

### `18_unsafe_ffi`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_unsafe` | `l00_unsafe_basics` | unsafe 边界 |
| `01_ffi` | `l00_ffi_mental` | FFI 心智 |
| `02_nomicon` | `l00_nomicon_checklist` | Nomicon 清单 |

### `19_perf_memory`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_bench` | `l00_criterion_mental` | 基准 |
| `01_alloc` | `l00_alloc_intuition` | 分配直觉 |
| `02_profile` | `l00_profiling_habits` | 剖析习惯 |

### `20_cloud_native`（3）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_container` | `l00_container_mental` | 容器 |
| `01_health` | `l00_health_checks` | 健康检查 |
| `02_config` | `l00_config_secrets` | 配置密钥 |

### `21_architecture`（4）
| 目录 | 包/课 | 核心点 |
| --- | --- | --- |
| `00_ddd` | `l00_ddd_tactical` | DDD |
| `01_events` | `l00_domain_events` | 事件 |
| `02_adr` | `l00_adr_checklist` | ADR |
| `03_sre` | `l00_sre_checklist` | SRE |

## 全自动生成批次

1. 一次只做一个 `NN_xxx`。
2. 写课 → 更新阶段 README → 更新根 README 状态 → verify → 单独 commit。
3. 用户要求再 push。
4. 生成顺序：`00`→`15`，再 `16→17→18→19→20→21`。
