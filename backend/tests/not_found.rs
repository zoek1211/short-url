mod common;

use axum::http::{StatusCode, header};
use common::{get, post_json};

async fn assert_not_found_page(app: &axum::Router, path: &str) -> String {
    let res = get(app, path).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND, "path: {path}");
    assert_eq!(
        res.headers[header::CONTENT_TYPE],
        "text/html; charset=utf-8",
        "path: {path}"
    );
    let body = String::from_utf8(res.body).unwrap();
    assert!(body.contains("단축 주소"), "path: {path}");
    assert!(body.contains("존재하지 않"), "path: {path}");
    assert!(
        body.contains(r#"href="http://localhost:8080/""#),
        "path: {path}"
    );
    body
}

#[tokio::test]
async fn unknown_valid_number_returns_korean_404_page_linking_to_creation_page() {
    let app = common::test_app().await;
    assert_not_found_page(&app, "/999").await;
}

#[tokio::test]
async fn malformed_numbers_return_the_same_404_page() {
    let app = common::test_app().await;
    let expected = assert_not_found_page(&app, "/999").await;
    for path in [
        "/042",
        "/0",
        "/-1",
        "/+5",
        "/99999999999999999999",
        "/9223372036854775808",
        "/abc",
        "/%EC%9D%BC",
    ] {
        assert_eq!(assert_not_found_page(&app, path).await, expected, "{path}");
    }
}

#[tokio::test]
async fn query_string_on_short_url_is_ignored() {
    let app = common::test_app().await;
    let original = "https://example.com/a?x=1";
    let res = post_json(&app, "/api/links", &format!(r#"{{"url":"{original}"}}"#)).await;
    let number = res.json["number"].as_i64().unwrap();

    let res = get(&app, &format!("/{number}?utm=x")).await;
    assert_eq!(res.status, StatusCode::FOUND);
    assert_eq!(res.headers[header::LOCATION], original);
}

#[tokio::test]
async fn existing_number_still_redirects() {
    let app = common::test_app().await;
    let res = post_json(&app, "/api/links", r#"{"url":"https://example.com/z"}"#).await;
    let number = res.json["number"].as_i64().unwrap();
    let res = get(&app, &format!("/{number}")).await;
    assert_eq!(res.status, StatusCode::FOUND);
    assert_eq!(res.headers[header::LOCATION], "https://example.com/z");
}
