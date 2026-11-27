# `08_web_axum` | Axum / Tower Web 心智（纯 Rust 迷你实现）

- **目标**: 在 **不依赖 axum  crate、不 bind 真端口** 的前提下，用 std 迷你代码建立与生产框架一一对应的心智：**Router**、**Extractor**、**Tower 中间件**、**Handler→Service→Repo 分层**。
- **前置**: `07_async_tokio`（理解 async 后再学真 Axum 更顺）；本阶段代码均为**同步离线**演示，与 Tokio 课解耦。
- **环境**: rustup 最新稳定版；验证：

```bash
./scripts/verify_modules.sh 08_web_axum
```

- **推荐资料（明文 URL，已核验）**:
- https://docs.rs/axum/latest/axum/
- https://github.com/tokio-rs/axum
- https://docs.rs/tower/latest/tower/
- https://docs.rs/axum/latest/axum/extract/index.html
- https://docs.rs/axum/latest/axum/middleware/index.html
- https://www.youtube.com/@RustVideos

## 迷你心智 ↔ Axum 对照

| 本仓库迷你概念 | 真实 Axum / Tower | 在哪一课 |
| --- | --- | --- |
| `MiniRouter` + `HashMap<RouteKey, _>` | `Router::route` / `merge` / `fallback` | `00_router` |
| `FromRequest::extract` | `Path<T>` / `Query<T>` / `Json<T>` / `FromRequest` | `01_extractor` |
| `Middleware` 高阶函数 + `compose` | `tower::Layer` / `ServiceBuilder` / `.layer(...)` | `02_middleware` |
| `UserHandler` + `State` 式注入 `Arc<dyn UserService>` | `State<AppState>` + 业务 Service + sqlx Repo | `03_layers` |

学完四课后，阅读官方文档时应能直接「对号入座」：你看到 `Router::nest`，会想到本课的 `merge`；看到 `Json(user): Json<CreateUser>`，会想到 `FromRequest`。

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_router/src/main.rs](./00_router/src/main.rs) | 路由表、模式路径、`404` fallback | ✅ |
| `01_` | [01_extractor/src/main.rs](./01_extractor/src/main.rs) | `FromRequest`、Path/Query/Json/Header | ✅ |
| `02_` | [02_middleware/src/main.rs](./02_middleware/src/main.rs) | 洋葱中间件、auth/log 链 | ✅ |
| `03_` | [03_layers/src/main.rs](./03_layers/src/main.rs) | Handler / Service / Repo trait 分层 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 明文 URL |
| --- | --- |
| Axum 总览 | https://docs.rs/axum/latest/axum/ |
| Axum 源码仓库 | https://github.com/tokio-rs/axum |
| Tower（Service/Layer） | https://docs.rs/tower/latest/tower/ |
| Extractors | https://docs.rs/axum/latest/axum/extract/index.html |
| Middleware | https://docs.rs/axum/latest/axum/middleware/index.html |
| 视频频道 | https://www.youtube.com/@RustVideos |

**本目录总课数：4（已完成 4）**

## 学习顺序建议

1. `00_router`：先会「注册路由 + dispatch」，再打开 Axum 文档里的 `Router`。
2. `01_extractor`：理解 handler 参数从哪来，再学 `Json`/`State`。
3. `02_middleware`：弄清 `.layer` 的顺序（外→内），对照 `TraceLayer`。
4. `03_layers`：固定分层习惯，再到 `09_persistence` 换真 Repo。

## 下一步

真机练习（可选，非本仓库 verify 要求）：新建独立 crate，`cargo add axum tokio`，写 `GET /health` 对照本课 `MiniRouter::dispatch` 的行为。