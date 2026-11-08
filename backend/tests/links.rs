mod common;

use axum::http::{StatusCode, header};
use common::{get, post_json};

#[tokio::test]
async fn create_then_redirect_preserves_original_url() {
    let app = common::test_app().await;
    let original = "https://example.com/a/b%20c?x=1&y=2#frag";

    let res = post_json(&app, "/api/links", &format!(r#"{{"url":"  {original}  "}}"#)).await;
    assert_eq!(res.status, StatusCode::CREATED);
    let number = res.json["number"].as_i64().unwrap();
    assert_eq!(res.json["short_url"], format!("http://localhost:8080/{number}"));

    let res = get(&app, &format!("/{number}")).await;
    assert_eq!(res.status, StatusCode::FOUND);
    assert_eq!(res.headers[header::LOCATION], original);
}

#[tokio::test]
async fn non_http_url_is_rejected() {
    let app = common::test_app().await;
    let res = post_json(&app, "/api/links", r#"{"url":"ftp://example.com/x"}"#).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert!(res.json["message"].is_string());
}

#[tokio::test]
async fn unknown_number_is_not_found() {
    let app = common::test_app().await;
    assert_eq!(get(&app, "/999").await.status, StatusCode::NOT_FOUND);
}
