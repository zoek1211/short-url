mod common;

use axum::http::{StatusCode, header};
use common::{app_for, get, post_json, test_pool};
use std::collections::HashSet;
use tokio::task::JoinSet;

fn body(url: &str) -> String {
    serde_json::json!({ "url": url }).to_string()
}

#[tokio::test]
async fn same_url_twice_returns_existing_link_with_200() {
    let app = common::test_app().await;
    let first = post_json(&app, "/api/links", &body("https://example.com/dup")).await;
    assert_eq!(first.status, StatusCode::CREATED);
    assert_eq!(first.json["existing"], false);

    // Surrounding whitespace is trimmed, so this is the same 원본 URL.
    let second = post_json(&app, "/api/links", &body("  https://example.com/dup \n")).await;
    assert_eq!(second.status, StatusCode::OK);
    assert_eq!(second.json["existing"], true);
    assert_eq!(second.json["number"], first.json["number"]);
    assert_eq!(second.json["short_url"], first.json["short_url"]);

    let number = first.json["number"].as_i64().unwrap();
    let res = get(&app, &format!("/{number}")).await;
    assert_eq!(res.headers[header::LOCATION], "https://example.com/dup");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_different_urls_created_concurrently_get_distinct_working_numbers() {
    let app = common::test_app().await;
    let mut set = JoinSet::new();
    for i in 0..50 {
        let app = app.clone();
        set.spawn(async move {
            let url = format!("https://example.com/concurrent/{i}");
            let res = post_json(&app, "/api/links", &body(&url)).await;
            (url, res)
        });
    }
    let mut seen = HashSet::new();
    let mut created = Vec::new();
    while let Some(r) = set.join_next().await {
        let (url, res) = r.unwrap();
        assert_eq!(res.status, StatusCode::CREATED, "{url}");
        assert_eq!(res.json["existing"], false);
        let number = res.json["number"].as_i64().unwrap();
        assert!(seen.insert(number), "duplicate number {number}");
        created.push((url, number));
    }
    assert_eq!(seen.len(), 50);
    for (url, number) in created {
        let res = get(&app, &format!("/{number}")).await;
        assert_eq!(res.status, StatusCode::FOUND);
        assert_eq!(res.headers[header::LOCATION], url.as_str());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn same_url_created_concurrently_yields_one_number_and_one_row() {
    let pool = test_pool().await;
    let app = app_for(pool.clone());
    let mut set = JoinSet::new();
    for _ in 0..50 {
        let app = app.clone();
        set.spawn(async move {
            post_json(&app, "/api/links", &body("https://example.com/same")).await
        });
    }
    let mut numbers = HashSet::new();
    let mut created = 0;
    while let Some(r) = set.join_next().await {
        let res = r.unwrap();
        match res.status {
            StatusCode::CREATED => created += 1,
            StatusCode::OK => assert_eq!(res.json["existing"], true),
            other => panic!("unexpected status {other}"),
        }
        numbers.insert(res.json["number"].as_i64().unwrap());
    }
    assert_eq!(created, 1, "exactly one request creates the link");
    assert_eq!(numbers.len(), 1, "numbers: {numbers:?}");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM links")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}
