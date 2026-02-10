//! Router integration tests using tower::ServiceExt::oneshot()
//!
//! These tests verify HTTP routing and handler behavior without spinning up a real server.

mod common;

use common::{CreateUserRequest, CreateUserResponse, GetUserResponse, MyServerApp};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;

/// Helper to create a router for testing
fn test_router() -> axum::Router {
    let app = Arc::new(MyServerApp {});
    app.router()
}

/// Helper to extract body bytes from a response
async fn body_bytes(body: Body) -> Vec<u8> {
    body.collect().await.unwrap().to_bytes().to_vec()
}

/// Helper to parse JSON response body
async fn json_body<T: serde::de::DeserializeOwned>(body: Body) -> T {
    let bytes = body_bytes(body).await;
    serde_json::from_slice(&bytes).unwrap()
}

// ── POST /users tests ───────────────────────────────────────────────────

#[tokio::test]
async fn test_create_user_success() {
    let router = test_router();

    let request_body = serde_json::to_string(&CreateUserRequest {
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
    })
    .unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Body::from(request_body))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: CreateUserResponse = json_body(response.into_body()).await;
    assert_eq!(body.name, "Bob");
    assert_eq!(body.id.0, 42); // The example implementation returns id 42
}

#[tokio::test]
async fn test_create_user_invalid_json() {
    let router = test_router();

    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Body::from("not valid json"))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Axum returns 400 Bad Request for JSON parse errors
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_user_missing_content_type() {
    let router = test_router();

    let request_body = serde_json::to_string(&CreateUserRequest {
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
    })
    .unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .body(Body::from(request_body))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Axum returns 415 Unsupported Media Type when content-type is missing
    assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}

#[tokio::test]
async fn test_create_user_empty_body() {
    let router = test_router();

    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Empty body should fail JSON parsing
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ── GET /users/{id} tests ───────────────────────────────────────────────

#[tokio::test]
async fn test_get_user_success() {
    let router = test_router();

    let request = Request::builder()
        .method("GET")
        .uri("/users/42")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: GetUserResponse = json_body(response.into_body()).await;
    assert_eq!(body.id.0, 42);
    assert_eq!(body.name, "Alice");
    assert_eq!(body.email, "alice@example.com");
}

#[tokio::test]
async fn test_get_user_invalid_id() {
    let router = test_router();

    let request = Request::builder()
        .method("GET")
        .uri("/users/not-a-number")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Invalid path parameter should return 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_user_with_different_ids() {
    let router = test_router();

    for id in [1, 100, 999] {
        let request = Request::builder()
            .method("GET")
            .uri(format!("/users/{}", id))
            .body(Body::empty())
            .unwrap();

        let response = router.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: GetUserResponse = json_body(response.into_body()).await;
        assert_eq!(body.id.0, id);
    }
}

// ── DELETE /users/{id} tests ────────────────────────────────────────────

#[tokio::test]
async fn test_delete_user_success() {
    let router = test_router();

    let request = Request::builder()
        .method("DELETE")
        .uri("/users/42")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // DELETE returns empty body (unit type serializes to null)
    let bytes = body_bytes(response.into_body()).await;
    let body_str = String::from_utf8(bytes).unwrap();
    assert!(body_str == "null" || body_str.is_empty());
}

#[tokio::test]
async fn test_delete_user_invalid_id() {
    let router = test_router();

    let request = Request::builder()
        .method("DELETE")
        .uri("/users/invalid")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ── 404 Not Found tests ─────────────────────────────────────────────────

#[tokio::test]
async fn test_404_unknown_path() {
    let router = test_router();

    let request = Request::builder()
        .method("GET")
        .uri("/unknown/path")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_404_wrong_method() {
    let router = test_router();

    // POST to /users/{id} should not exist (only GET and DELETE)
    let request = Request::builder()
        .method("POST")
        .uri("/users/42")
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // Axum returns 405 Method Not Allowed when path matches but method doesn't
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_404_partial_path() {
    let router = test_router();

    let request = Request::builder()
        .method("GET")
        .uri("/users")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    // GET /users doesn't exist (only POST /users)
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// ── Content-Type tests ──────────────────────────────────────────────────

#[tokio::test]
async fn test_response_content_type_json() {
    let router = test_router();

    let request = Request::builder()
        .method("GET")
        .uri("/users/1")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    assert!(
        content_type.contains("application/json"),
        "Expected JSON content type, got: {}",
        content_type
    );
}

// ── Edge case tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_user_with_unicode() {
    let router = test_router();

    let request_body = serde_json::to_string(&CreateUserRequest {
        name: "日本語ユーザー".to_string(),
        email: "user@例え.jp".to_string(),
    })
    .unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Body::from(request_body))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: CreateUserResponse = json_body(response.into_body()).await;
    assert_eq!(body.name, "日本語ユーザー");
}

#[tokio::test]
async fn test_create_user_with_special_characters() {
    let router = test_router();

    let request_body = serde_json::to_string(&CreateUserRequest {
        name: "O'Brien & \"Test\"".to_string(),
        email: "test+tag@example.com".to_string(),
    })
    .unwrap();

    let request = Request::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Body::from(request_body))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: CreateUserResponse = json_body(response.into_body()).await;
    assert_eq!(body.name, "O'Brien & \"Test\"");
}

#[tokio::test]
async fn test_get_user_max_id() {
    let router = test_router();

    let request = Request::builder()
        .method("GET")
        .uri(format!("/users/{}", u32::MAX))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body: GetUserResponse = json_body(response.into_body()).await;
    assert_eq!(body.id.0, u32::MAX);
}
