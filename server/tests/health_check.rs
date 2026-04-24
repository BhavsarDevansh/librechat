//! Integration tests for the Axum health check server (Issue #3).

use axum::body::Body;
use axum::http::{self, header, Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use server::app;
use server::state::AppState;

/// Helper: build the app with default state for testing.
fn test_app() -> axum::Router {
    app(AppState::new())
}

// ---- Health endpoint tests ----

#[tokio::test]
async fn test_health_endpoint_returns_200_ok() {
    let app = test_app();
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_endpoint_returns_json_status_ok() {
    let app = test_app();
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");
    let body = resp
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn test_health_endpoint_content_type_is_json() {
    let app = test_app();
    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");
    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("content-type header missing");
    assert!(
        ct.to_str()
            .expect("content-type not readable")
            .starts_with("application/json"),
        "content-type should be application/json, got: {ct:?}"
    );
}

#[tokio::test]
async fn test_cors_preflight_allows_default_local_origin() {
    let app = test_app();
    let req = Request::builder()
        .method(http::Method::OPTIONS)
        .uri("/health")
        .header(header::ORIGIN, "http://localhost:8080")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");
    let allow_origin = resp
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .expect("access-control-allow-origin header missing");
    assert_eq!(
        allow_origin.to_str().expect("header value"),
        "http://localhost:8080"
    );
}

#[tokio::test]
async fn test_cors_on_get_request_for_default_local_origin() {
    let app = test_app();
    let req = Request::builder()
        .uri("/health")
        .header(header::ORIGIN, "http://localhost:8080")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");
    let allow_origin = resp
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .expect("access-control-allow-origin header missing");
    assert_eq!(
        allow_origin.to_str().expect("header value"),
        "http://localhost:8080"
    );
}

#[tokio::test]
async fn test_cors_does_not_allow_unlisted_origin() {
    let app = test_app();
    let req = Request::builder()
        .uri("/health")
        .header(header::ORIGIN, "http://example.com")
        .body(Body::empty())
        .expect("build request");
    let resp = app.oneshot(req).await.expect("oneshot");

    assert!(
        resp.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none(),
        "unexpected access-control-allow-origin header for unlisted origin"
    );
}
