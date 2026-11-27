/*
[主题]: Extractor 心智 —— 从假 Request 里「按类型取出」参数
[难度]: Medium
[前置]: 本阶段 `00_router`、02_structs_enums、04_traits_generics

一、主题说明
- Axum handler 写法：`async fn show(Path(id): Path<u64>, Json(body): Json<CreateUser>) -> impl IntoResponse`。
- 框架在背后做的事：按参数类型依次 **extract**，失败则返回 4xx（如 400/422）。
- 本课：定义 `FromRequest` trait，实现 `PathParam`、`JsonBody`、`QueryParams`（极简 JSON 解析，无 serde 依赖也可跑）。

二、写法对比
1. 直观版: handler 里手动 `split`、`parse`、拆字符串。
2. 进阶版: 每个类型实现自己的 `extract`，handler 只拿强类型。

三、真实用途与对照
- Axum Extractors：https://docs.rs/axum/latest/axum/extract/index.html
- `Path` / `Query` / `Json` / `State` 都是同一套机制的不同实现。
资料: https://docs.rs/axum/latest/axum/extract/index.html
      https://docs.rs/axum/latest/axum/
      https://www.youtube.com/@RustVideos
*/

use std::collections::HashMap;

#[derive(Clone, Debug)]
struct Request {
    path: String,
    query: String,
    body: String,
    headers: HashMap<String, String>,
}

#[derive(Debug, PartialEq)]
struct ExtractError {
    code: u16,
    message: String,
}

/// 对照 axum::extract::FromRequest（同步简化版）
trait FromRequest: Sized {
    fn extract(req: &Request) -> Result<Self, ExtractError>;
}

#[derive(Clone, Debug, PartialEq)]
struct PathParam {
    pub id: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct QueryParams {
    pub page: u32,
    pub q: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct JsonBody {
    pub name: String,
    pub age: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct HeaderContentType(String);

/// 从路径末段取 id：`/users/42` -> 42
impl FromRequest for PathParam {
    fn extract(req: &Request) -> Result<Self, ExtractError> {
        let last = req
            .path
            .rsplit('/')
            .find(|s| !s.is_empty())
            .ok_or_else(|| ExtractError {
                code: 400,
                message: "path empty".into(),
            })?;
        let id: u64 = last.parse().map_err(|_| ExtractError {
            code: 400,
            message: format!("invalid id segment: {last}"),
        })?;
        Ok(PathParam { id })
    }
}

/// 解析 `?page=2&q=rust`（不做 URL 编码，教学够用）
impl FromRequest for QueryParams {
    fn extract(req: &Request) -> Result<Self, ExtractError> {
        let mut page = 1u32;
        let mut q = None;
        if !req.query.is_empty() {
            for pair in req.query.split('&') {
                let mut kv = pair.splitn(2, '=');
                let key = kv.next().unwrap_or("");
                let val = kv.next().unwrap_or("");
                match key {
                    "page" => {
                        page = val.parse().map_err(|_| ExtractError {
                            code: 400,
                            message: "bad page".into(),
                        })?;
                    }
                    "q" => q = Some(val.to_string()),
                    _ => {}
                }
            }
        }
        Ok(QueryParams { page, q })
    }
}

/// 极简 JSON：只认 `{"name":"...","age":N}` 形态（对照 serde_json + Json<T>）
impl FromRequest for JsonBody {
    fn extract(req: &Request) -> Result<Self, ExtractError> {
        let body = req.body.trim();
        if !body.starts_with('{') || !body.ends_with('}') {
            return Err(ExtractError {
                code: 400,
                message: "body not json object".into(),
            });
        }
        let mut name = None;
        let mut age = None;
        for part in body.trim_matches(|c| c == '{' || c == '}').split(',') {
            let part = part.trim();
            if let Some(rest) = part.strip_prefix("\"name\":") {
                let v = rest.trim().trim_matches('"');
                name = Some(v.to_string());
            } else if let Some(rest) = part.strip_prefix("\"age\":") {
                let v: u32 = rest.trim().parse().map_err(|_| ExtractError {
                    code: 400,
                    message: "bad age".into(),
                })?;
                age = Some(v);
            }
        }
        Ok(JsonBody {
            name: name.ok_or_else(|| ExtractError {
                code: 400,
                message: "missing name".into(),
            })?,
            age: age.ok_or_else(|| ExtractError {
                code: 400,
                message: "missing age".into(),
            })?,
        })
    }
}

impl FromRequest for HeaderContentType {
    fn extract(req: &Request) -> Result<Self, ExtractError> {
        let v = req
            .headers
            .get("content-type")
            .cloned()
            .ok_or_else(|| ExtractError {
                code: 415,
                message: "missing content-type".into(),
            })?;
        Ok(HeaderContentType(v))
    }
}

/// 直观版：全在 handler 里拆
fn create_user_naive(req: &Request) -> Result<String, ExtractError> {
    let id = req
        .path
        .rsplit('/')
        .next()
        .unwrap_or("0")
        .parse::<u64>()
        .map_err(|_| ExtractError {
            code: 400,
            message: "bad id".into(),
        })?;
    if !req.body.contains("\"name\"") {
        return Err(ExtractError {
            code: 400,
            message: "bad json".into(),
        });
    }
    Ok(format!("created user slot {id}"))
}

/// 进阶版：对照「多 Extractor 参数列表」
fn create_user_better(req: &Request) -> Result<String, ExtractError> {
    let PathParam { id } = PathParam::extract(req)?;
    let JsonBody { name, age } = JsonBody::extract(req)?;
    let HeaderContentType(ct) = HeaderContentType::extract(req)?;
    if !ct.contains("json") {
        return Err(ExtractError {
            code: 415,
            message: "need application/json".into(),
        });
    }
    Ok(format!("user {id}: {name}, age {age}"))
}

fn run_tests() {
    let req = Request {
        path: "/users/99".into(),
        query: "page=3&q=axum".into(),
        body: r#"{"name":"Lin","age":20}"#.into(),
        headers: HashMap::from([("content-type".into(), "application/json".into())]),
    };

    assert_eq!(PathParam::extract(&req).unwrap().id, 99);
    let q = QueryParams::extract(&req).unwrap();
    assert_eq!(q.page, 3);
    assert_eq!(q.q.as_deref(), Some("axum"));
    assert_eq!(
        JsonBody::extract(&req).unwrap(),
        JsonBody {
            name: "Lin".into(),
            age: 20
        }
    );

    assert!(create_user_naive(&req).unwrap().contains("99"));
    assert_eq!(
        create_user_better(&req).unwrap(),
        "user 99: Lin, age 20"
    );

    let bad_path = Request {
        path: "/users/xx".into(),
        query: String::new(),
        body: String::new(),
        headers: HashMap::new(),
    };
    assert!(PathParam::extract(&bad_path).is_err());

    println!("✅ Extractor 心智（迷你 Axum 对照）所有测试用例通过！");
}

fn bootstrap() {
    println!("\n\n************ Extractor 心智 ************\n");
    println!("对照 Axum：Path / Query / Json / TypedHeader —— 本课用 trait 离线提取。\n");

    let req = Request {
        path: "/users/1".into(),
        query: "page=1".into(),
        body: r#"{"name":"A","age":18}"#.into(),
        headers: HashMap::from([("content-type".into(), "application/json".into())]),
    };
    match create_user_better(&req) {
        Ok(msg) => println!("handler 结果: {msg}"),
        Err(e) => println!("extract 失败 {}: {}", e.code, e.message),
    }

    run_tests();
}

fn main() {
    bootstrap();
}