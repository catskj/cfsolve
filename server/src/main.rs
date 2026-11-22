use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use cfsolve::{Browser, Options, PageSource, Turnstile, Widget};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

struct App {
    browser: Browser,
    api_key: String,
}

#[derive(Deserialize)]
struct Req {
    url: String,
    sitekey: Option<String>,
    action: Option<String>,
    cdata: Option<String>,
    proxy: Option<String>,
}

type Resp = (StatusCode, Json<Value>);

fn err(code: StatusCode, msg: String) -> Resp {
    (code, Json(json!({ "success": false, "error": msg })))
}

async fn solve(State(app): State<Arc<App>>, headers: HeaderMap, Json(req): Json<Req>) -> Resp {
    let ok = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == app.api_key)
        .unwrap_or(false);
    if !ok {
        return err(StatusCode::UNAUTHORIZED, "bad or missing x-api-key".into());
    }

    let url = match req.url.parse() {
        Ok(u) => u,
        Err(e) => return err(StatusCode::BAD_REQUEST, format!("bad url: {e}")),
    };

    // With a sitekey: render that widget on a blank page at the URL.
    // Without: fetch the page and solve the first widget on it.
    let (page, widget) = match req.sitekey {
        Some(sk) => (
            PageSource::Blank,
            Some(Widget { sitekey: sk.into(), action: req.action, cdata: req.cdata }),
        ),
        None => (PageSource::Navigate, None),
    };

    let options = Options { proxy: req.proxy, ..Default::default() };
    match app.browser.solve_turnstile(Turnstile { url, page, widget }, &options).await {
        Ok(token) => (StatusCode::OK, Json(json!({ "success": true, "token": token.to_string() }))),
        Err(e) => err(StatusCode::BAD_GATEWAY, format!("{e}")),
    }
}

#[tokio::main]
async fn main() {
    let api_key = std::env::var("API_KEY").expect("set API_KEY env var");
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "profiles/safari-26-ios.json".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());

    let browser = Browser::read(&profile).expect("failed to load profile");
    let app = Arc::new(App { browser, api_key });

    let router = Router::new()
        .route("/", get(|| async { "ok" }))
        .route("/solve", post(solve))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

