# `12_grpc_tonic` | 微服务与 tonic（纯 Rust 迷你实现）

- **目标**: 在 **不连真网络、不依赖 tonic crate** 的前提下，建立与生产 gRPC 栈一一对应的心智：**服务拆分**、**Unary RPC + Status + Stub/Server**、**API 网关（鉴权/限流/聚合）**。
- **前置**: `08_web_axum`（Router/中间件）、`09_persistence/00_repo`（trait 边界）；学完本阶段再 `cargo add tonic prost` 上真 proto 更顺。
- **环境**: rustup 最新稳定版；验证：

```bash
./scripts/verify_modules.sh 12_grpc_tonic
```

- **推荐资料（明文 URL，已核验）**:
- https://docs.rs/tonic/latest/tonic/
- https://github.com/hyperium/tonic
- https://www.youtube.com/@RustVideos

## 迷你心智 ↔ tonic 对照

| 本仓库迷你概念 | 真实 tonic / gRPC | 在哪一课 |
| --- | --- | --- |
| `UserService` / `OrderService` 边界 + `UserLookup` trait | 独立二进制 + proto `service` + Client stub | `00_split` |
| `full_method` 路径 + `RpcStatus` + `MiniStub::invoke` | `tonic::Status`、`UserServiceClient`、HTTP/2 帧 | `01_tonic` |
| `MiniGateway::handle` + Bearer 鉴权 + BFF 聚合 | Axum 公网入口 + tonic 内网 Client + Envoy/Kong | `02_gateway` |

真 tonic 流程简述：写 `.proto` → `tonic_build` 生成 Rust → `Server::builder().add_service(...).serve(addr)` + 客户端 `connect`。

## 章节进度

| 编号 | 文件 | 核心点 | 状态 |
| --- | --- | --- | --- |
| `00_` | [00_split/src/main.rs](./00_split/src/main.rs) | 单体 vs 拆分、trait 客户端、注册表 | ✅ |
| `01_` | [01_tonic/src/main.rs](./01_tonic/src/main.rs) | Unary RPC、Status、Stub/Server、假序列化 | ✅ |
| `02_` | [02_gateway/src/main.rs](./02_gateway/src/main.rs) | HTTP 入口、鉴权、限流、dashboard 聚合 | ✅ |

## 本阶段知识点资料包（明文 URL）

| 知识点 | 明文 URL |
| --- | --- |
| tonic API 文档 | https://docs.rs/tonic/latest/tonic/ |
| tonic 源码与示例 | https://github.com/hyperium/tonic |
| Tower（网关常用 Layer） | https://docs.rs/tower/latest/tower/ |
| 视频频道 | https://www.youtube.com/@RustVideos |

**本目录总课数：3（已完成 3）**

## 学习顺序建议

1. `00_split`：先弄清「为什么拆」与「跨服务只走契约」，避免回到单体大 HashMap。
2. `01_tonic`：记住 gRPC **方法全名** 与 **Status**；再打开 tonic 文档里的 `Code` 枚举。
3. `02_gateway`：理解对外 HTTP、对内 gRPC；聚合接口要想到超时与部分失败。

## 下一步（可选，非 verify 要求）

新建独立 workspace crate：`cargo add tonic prost tokio`，用 [tonic 仓库](https://github.com/hyperium/tonic) 的 helloworld 示例对照本课 `MiniGrpcServer::invoke`。再继续 `13_testing` 给 gRPC handler 写表驱动测试。