mod common;

use axum::http::StatusCode;
use common::get;

#[tokio::test]
async fn healthz_returns_200_ok() {
    let app = common::test_app().await;
    let res = get(&app, "/healthz").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body, b"ok");
}
