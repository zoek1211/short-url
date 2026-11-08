#![allow(dead_code)]

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Method, Request, StatusCode, header},
};
use backend::{Config, MIGRATOR, router};
use http_body_util::BodyExt;
use sqlx::{Connection, PgConnection, PgPool};
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;
use tower::ServiceExt;

static PG: OnceCell<(ContainerAsync<Postgres>, String)> = OnceCell::const_new();

async fn admin_url() -> &'static str {
    let (_, url) = PG
        .get_or_init(|| async {
            let c = Postgres::default().start().await.expect("start postgres (is Docker running?)");
            let port = c.get_host_port_ipv4(5432).await.unwrap();
            (c, format!("postgres://postgres:postgres@127.0.0.1:{port}"))
        })
        .await;
    url
}

/// Fresh, migrated database per call.
pub async fn test_pool() -> PgPool {
    let admin = admin_url().await;
    let db = format!("t_{}", uuid::Uuid::new_v4().simple());
    let mut conn = PgConnection::connect(&format!("{admin}/postgres")).await.unwrap();
    sqlx::query(&format!("CREATE DATABASE {db}")).execute(&mut conn).await.unwrap();
    let pool = PgPool::connect(&format!("{admin}/{db}")).await.unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    pool
}

pub fn app_for(pool: PgPool) -> Router {
    router(pool, Config { public_base_url: "http://localhost:8080".into() })
}

pub async fn test_app() -> Router {
    app_for(test_pool().await)
}

pub struct Res {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub json: serde_json::Value,
}

async fn send(app: &Router, req: Request<Body>) -> Res {
    let res = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    let body = body.collect().await.unwrap().to_bytes().to_vec();
    let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    Res { status: parts.status, headers: parts.headers, body, json }
}

pub async fn get(app: &Router, path: &str) -> Res {
    send(app, Request::get(path).body(Body::empty()).unwrap()).await
}

pub async fn post_json(app: &Router, path: &str, body: &str) -> Res {
    let req = Request::builder()
        .method(Method::POST)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_owned()))
        .unwrap();
    send(app, req).await
}
