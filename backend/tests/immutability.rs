mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use common::{app_for, get, post_json, test_app, test_pool};
use tower::ServiceExt;

fn body(url: &str) -> String {
    serde_json::json!({ "url": url }).to_string()
}

async fn with_method(app: &axum::Router, method: Method, path: &str) -> (StatusCode, bool) {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    (res.status(), res.headers().contains_key(header::ALLOW))
}

#[tokio::test]
async fn non_get_methods_on_number_and_link_paths_are_405_and_link_survives() {
    let app = test_app().await;
    let created = post_json(&app, "/api/links", &body("https://example.com/keep")).await;
    let n = created.json["number"].as_i64().unwrap();

    for method in [Method::PUT, Method::PATCH, Method::DELETE, Method::POST] {
        let (status, allow) = with_method(&app, method.clone(), &format!("/{n}")).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} /{n}");
        assert!(allow, "{method} /{n} missing Allow header");
    }
    for method in [Method::PUT, Method::PATCH, Method::DELETE] {
        let path = format!("/api/links/{n}");
        let (status, allow) = with_method(&app, method.clone(), &path).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
        assert!(allow, "{method} {path} missing Allow header");
    }
    for method in [Method::PUT, Method::PATCH, Method::DELETE] {
        let (status, _) = with_method(&app, method, "/api/links").await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    let res = get(&app, &format!("/{n}")).await;
    assert_eq!(res.status, StatusCode::FOUND);
    assert_eq!(res.headers[header::LOCATION], "https://example.com/keep");
}

#[tokio::test]
async fn get_on_link_api_path_returns_the_link_or_404_json() {
    let app = test_app().await;
    let created = post_json(&app, "/api/links", &body("https://example.com/one")).await;
    let n = created.json["number"].as_i64().unwrap();

    let res = get(&app, &format!("/api/links/{n}")).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.json["number"], n);
    assert_eq!(res.json["short_url"], created.json["short_url"]);
    assert_eq!(res.json["original_url"], "https://example.com/one");

    let res = get(&app, "/api/links/999999").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert!(res.json["message"].is_string());
}

async fn link_count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM links")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn extra_body_fields_are_400_and_create_nothing() {
    let pool = test_pool().await;
    let app = app_for(pool.clone());
    for raw in [
        r#"{"url": "https://example.com/x", "number": 1}"#,
        r#"{"url": "https://example.com/x", "original_url": "https://example.com/y"}"#,
    ] {
        let res = post_json(&app, "/api/links", raw).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{raw}");
        let msg = res.json["message"].as_str().expect("ErrorResponse message");
        assert!(msg.chars().any(|c| ('가'..='힣').contains(&c)), "{msg}");
    }
    assert_eq!(link_count(&pool).await, 0);
}

#[tokio::test]
async fn malformed_json_and_missing_url_are_400_with_korean_message() {
    let pool = test_pool().await;
    let app = app_for(pool.clone());
    for raw in ["{not json", "", "{}", r#"{"url": 5}"#, "[]"] {
        let res = post_json(&app, "/api/links", raw).await;
        assert_eq!(res.status, StatusCode::BAD_REQUEST, "{raw:?}");
        let msg = res.json["message"].as_str().expect("ErrorResponse message");
        assert!(msg.chars().any(|c| ('가'..='힣').contains(&c)), "{msg}");
    }
    assert_eq!(link_count(&pool).await, 0);
}

#[tokio::test]
async fn direct_sql_update_delete_truncate_are_rejected_and_row_is_unchanged() {
    let pool = test_pool().await;
    let app = app_for(pool.clone());
    let created = post_json(&app, "/api/links", &body("https://example.com/sql")).await;
    let n = created.json["number"].as_i64().unwrap();

    assert!(
        sqlx::query("UPDATE links SET original_url = 'https://evil.test/' WHERE number = $1")
            .bind(n)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE links SET number = 999 WHERE number = $1")
            .bind(n)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM links WHERE number = $1")
            .bind(n)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(sqlx::query("TRUNCATE links").execute(&pool).await.is_err());

    let rows: Vec<(i64, String)> = sqlx::query_as("SELECT number, original_url FROM links")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows, vec![(n, "https://example.com/sql".to_string())]);

    // Re-submitting the same URL (ON CONFLICT DO NOTHING) still works.
    let again = post_json(&app, "/api/links", &body("https://example.com/sql")).await;
    assert_eq!(again.status, StatusCode::OK);
    assert_eq!(again.json["number"], n);
}

#[tokio::test]
async fn links_survive_a_restart_with_a_new_router_and_pool() {
    let pool = test_pool().await;
    let app = app_for(pool.clone());
    let created = post_json(&app, "/api/links", &body("https://example.com/persist")).await;
    let n = created.json["number"].as_i64().unwrap();

    let opts = (*pool.connect_options()).clone();
    drop(app);
    pool.close().await;

    let new_pool = sqlx::PgPool::connect_with(opts).await.unwrap();
    let restarted = app_for(new_pool);
    let res = get(&restarted, &format!("/{n}")).await;
    assert_eq!(res.status, StatusCode::FOUND);
    assert_eq!(res.headers[header::LOCATION], "https://example.com/persist");
}
