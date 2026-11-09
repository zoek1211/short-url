mod common;

use axum::http::StatusCode;
use common::{get_with, post_json_with};

const EN_COOKIE: &[(&str, &str)] = &[("cookie", "lang=en")];

#[tokio::test]
async fn lang_cookie_en_gives_english_validation_message() {
    let app = common::test_app().await;
    let res = post_json_with(
        &app,
        "/api/links",
        r#"{"url":"ftp://example.com"}"#,
        EN_COOKIE,
    )
    .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        res.json["message"],
        "The original URL must start with http or https."
    );

    let res = post_json_with(&app, "/api/links", r#"{"url":"x","extra":1}"#, EN_COOKIE).await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST);
    assert!(
        res.json["message"]
            .as_str()
            .unwrap()
            .starts_with("The request body")
    );
}

#[tokio::test]
async fn accept_language_en_gives_english_404_page_and_json() {
    let app = common::test_app().await;
    let h = &[("accept-language", "en-US,en;q=0.9")];
    let res = get_with(&app, "/999", h).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    let body = String::from_utf8(res.body).unwrap();
    assert!(body.contains(r#"<html lang="en">"#));
    assert!(body.contains("Short URL not found"));
    assert!(
        body.contains("lang=ko"),
        "switch button sets the Korean cookie"
    );

    let res = get_with(&app, "/api/links/999", h).await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.json["message"], "This link does not exist.");
}

#[tokio::test]
async fn cookie_overrides_accept_language() {
    let app = common::test_app().await;
    let res = get_with(
        &app,
        "/999",
        &[("cookie", "lang=ko"), ("accept-language", "en")],
    )
    .await;
    let body = String::from_utf8(res.body).unwrap();
    assert!(body.contains(r#"<html lang="ko">"#));
    assert!(body.contains("단축 주소를 찾을 수 없습니다"));
    assert!(body.contains("lang=en"));
}

#[tokio::test]
async fn success_response_is_the_same_in_both_languages() {
    let app = common::test_app().await;
    let body = r#"{"url":"https://example.com/lang"}"#;
    let ko = post_json_with(&app, "/api/links", body, &[]).await;
    let en = post_json_with(&app, "/api/links", body, EN_COOKIE).await;
    assert_eq!(ko.status, StatusCode::CREATED);
    assert_eq!(en.status, StatusCode::OK);
    assert_eq!(ko.json["number"], en.json["number"]);
}
