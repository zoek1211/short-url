mod common;

use axum::http::{StatusCode, header};
use common::{get, post_json};

#[tokio::test]
async fn create_then_redirect_preserves_original_url() {
    let app = common::test_app().await;
    let original = "https://example.com/a/b%20c?x=1&y=2#frag";

    let res = post_json(
        &app,
        "/api/links",
        &format!(r#"{{"url":"  {original}  "}}"#),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let number = res.json["number"].as_i64().unwrap();
    assert_eq!(
        res.json["short_url"],
        format!("http://localhost:8080/{number}")
    );

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

async fn assert_rejected(app: &axum::Router, url: &str) -> String {
    let res = post_json(
        app,
        "/api/links",
        &serde_json::json!({ "url": url }).to_string(),
    )
    .await;
    assert_eq!(res.status, StatusCode::BAD_REQUEST, "url: {url:?}");
    let message = res.json["message"]
        .as_str()
        .expect("message string")
        .to_string();
    assert!(!message.is_empty());
    message
}

#[tokio::test]
async fn each_rejection_reason_has_a_distinct_message() {
    let app = common::test_app().await;
    let long = format!("https://example.com/{}", "a".repeat(2049));
    let one_per_reason = [
        "ftp://example.com/x",             // scheme
        "https://",                        // no host
        "https://example.com/a b",         // inner whitespace
        "https://user:pass@example.com/x", // userinfo
        "http://localhost:8080/1",         // own host
        long.as_str(),                     // too long
    ];
    let mut messages = Vec::new();
    for url in one_per_reason {
        messages.push(assert_rejected(&app, url).await);
    }
    let mut unique = messages.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), messages.len(), "messages: {messages:?}");
}

#[tokio::test]
async fn disallowed_scheme_is_rejected() {
    let app = common::test_app().await;
    for url in [
        "ftp://example.com/x",
        "javascript:alert(1)",
        "mailto:a@b.com",
        "example.com",
    ] {
        assert_rejected(&app, url).await;
    }
}

#[tokio::test]
async fn url_without_host_is_rejected() {
    let app = common::test_app().await;
    for url in ["https://", "http://:8080/", "https:///"] {
        assert_rejected(&app, url).await;
    }
}

#[tokio::test]
async fn url_with_inner_whitespace_or_control_chars_is_rejected() {
    let app = common::test_app().await;
    for url in [
        "https://example.com/a b",
        "https://example.com/a\tb",
        "https://example.com/a\nb",
        "https://example.com/a\u{0}b",
        "https://example.com/a\u{7f}b",
        "https://example.com/a\u{3000}b",
    ] {
        assert_rejected(&app, url).await;
    }
}

#[tokio::test]
async fn url_with_userinfo_is_rejected() {
    let app = common::test_app().await;
    for url in [
        "https://user:pass@example.com/",
        "https://user@example.com/",
        "https://:pass@example.com/",
    ] {
        assert_rejected(&app, url).await;
    }
}

#[tokio::test]
async fn url_over_2048_bytes_is_rejected_and_2048_passes() {
    let app = common::test_app().await;
    let prefix = "https://example.com/";

    let ok = format!("{prefix}{}", "a".repeat(2048 - prefix.len()));
    assert_eq!(ok.len(), 2048);
    let res = post_json(
        &app,
        "/api/links",
        &serde_json::json!({ "url": ok }).to_string(),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);

    let too_long = format!("{prefix}{}", "a".repeat(2049 - prefix.len()));
    assert_rejected(&app, &too_long).await;

    // 2048 chars but more than 2048 bytes (3 bytes per Korean syllable).
    let multibyte = format!("{prefix}{}", "가".repeat(2048 - prefix.len()));
    assert!(multibyte.chars().count() <= 2048 && multibyte.len() > 2048);
    assert_rejected(&app, &multibyte).await;
}

#[tokio::test]
async fn self_link_to_either_base_host_is_rejected() {
    let app = common::test_app().await;
    for url in [
        "http://localhost:8080/1",
        "https://localhost/1",
        "http://LOCALHOST:9999/x",
        "http://api.example.test:3000/1",
        "https://API.Example.Test/1",
    ] {
        assert_rejected(&app, url).await;
    }
    let res = post_json(
        &app,
        "/api/links",
        r#"{"url":"https://other.example.test/1"}"#,
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
}

#[tokio::test]
async fn surrounding_whitespace_is_trimmed_before_validation() {
    let app = common::test_app().await;
    let res = post_json(
        &app,
        "/api/links",
        &serde_json::json!({ "url": " \t\nhttps://example.com/x \n" }).to_string(),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let number = res.json["number"].as_i64().unwrap();
    let res = get(&app, &format!("/{number}")).await;
    assert_eq!(res.headers[header::LOCATION], "https://example.com/x");
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            out.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

#[tokio::test]
async fn non_ascii_url_is_accepted_and_redirects_with_ascii_location() {
    let app = common::test_app().await;
    let res = post_json(
        &app,
        "/api/links",
        &serde_json::json!({ "url": "https://ko.wikipedia.org/wiki/대한민국?q=한글#절" })
            .to_string(),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let number = res.json["number"].as_i64().unwrap();

    let res = get(&app, &format!("/{number}")).await;
    assert_eq!(res.status, StatusCode::FOUND);
    let location = res.headers[header::LOCATION]
        .to_str()
        .expect("ASCII Location");
    assert!(location.is_ascii());
    assert!(location.starts_with("https://ko.wikipedia.org/wiki/%"));
    assert_eq!(
        percent_decode(location),
        "https://ko.wikipedia.org/wiki/대한민국?q=한글#절"
    );
}

#[tokio::test]
async fn non_ascii_host_redirects_with_punycode() {
    let app = common::test_app().await;
    let res = post_json(
        &app,
        "/api/links",
        &serde_json::json!({ "url": "https://한글.example.com/" }).to_string(),
    )
    .await;
    assert_eq!(res.status, StatusCode::CREATED);
    let number = res.json["number"].as_i64().unwrap();
    let res = get(&app, &format!("/{number}")).await;
    assert_eq!(res.status, StatusCode::FOUND);
    assert!(
        res.headers[header::LOCATION]
            .to_str()
            .unwrap()
            .starts_with("https://xn--")
    );
}
