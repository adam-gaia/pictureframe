//! Client integration tests that spin up an actual server.
//!
//! These tests verify the generated HTTP client works correctly against a real server.

mod common;

use common::{
    CreateUserRequest, MyServerApp, MyServerAppClient, MyServerAppClientError, UserId,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Spawn a test server on a random port and return its address.
async fn spawn_server() -> SocketAddr {
    let app = Arc::new(MyServerApp {});
    let router = app.router();

    // Bind to port 0 to get a random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn the server in the background
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    addr
}

// ── Client create_user tests ────────────────────────────────────────────

#[tokio::test]
async fn test_client_create_user() {
    let addr = spawn_server().await;
    let client = MyServerAppClient::new(format!("http://{}", addr));

    let result = client
        .create_user(&CreateUserRequest {
            name: "TestUser".to_string(),
            email: "test@example.com".to_string(),
        })
        .await;

    let response = result.expect("create_user should succeed");
    assert_eq!(response.name, "TestUser");
    assert_eq!(response.id.0, 42);
}

#[tokio::test]
async fn test_client_create_user_unicode() {
    let addr = spawn_server().await;
    let client = MyServerAppClient::new(format!("http://{}", addr));

    let result = client
        .create_user(&CreateUserRequest {
            name: "ユーザー名".to_string(),
            email: "test@example.com".to_string(),
        })
        .await;

    let response = result.expect("create_user should succeed");
    assert_eq!(response.name, "ユーザー名");
}

// ── Client get_user tests ───────────────────────────────────────────────

#[tokio::test]
async fn test_client_get_user() {
    let addr = spawn_server().await;
    let client = MyServerAppClient::new(format!("http://{}", addr));

    let result = client.get_user(UserId(42)).await;

    let response = result.expect("get_user should succeed");
    assert_eq!(response.id.0, 42);
    assert_eq!(response.name, "Alice");
    assert_eq!(response.email, "alice@example.com");
}

#[tokio::test]
async fn test_client_get_user_different_ids() {
    let addr = spawn_server().await;
    let client = MyServerAppClient::new(format!("http://{}", addr));

    for id in [1, 100, 999] {
        let result = client.get_user(UserId(id)).await;
        let response = result.expect("get_user should succeed");
        assert_eq!(response.id.0, id);
    }
}

// ── Client delete_user tests ────────────────────────────────────────────

#[tokio::test]
async fn test_client_delete_user() {
    let addr = spawn_server().await;
    let client = MyServerAppClient::new(format!("http://{}", addr));

    let result = client.delete_user(UserId(42)).await;

    result.expect("delete_user should succeed");
}

// ── Client error handling tests ─────────────────────────────────────────

#[tokio::test]
async fn test_client_network_error() {
    // Connect to a port that nothing is listening on
    let client = MyServerAppClient::new("http://127.0.0.1:1");

    let result = client.get_user(UserId(1)).await;

    match result {
        Err(MyServerAppClientError::Request(_)) => {
            // Expected: network/connection error
        }
        Err(MyServerAppClientError::Api { status, body }) => {
            panic!("Expected Request error, got Api error: {} - {}", status, body);
        }
        Ok(_) => {
            panic!("Expected error, got success");
        }
    }
}

// ── Client with custom reqwest::Client ──────────────────────────────────

#[tokio::test]
async fn test_client_with_custom_client() {
    let addr = spawn_server().await;

    let custom_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    let client = MyServerAppClient::with_client(format!("http://{}", addr), custom_client);

    let result = client.get_user(UserId(1)).await;
    result.expect("get_user should succeed with custom client");
}

// ── Multiple sequential requests ────────────────────────────────────────

#[tokio::test]
async fn test_client_multiple_requests() {
    let addr = spawn_server().await;
    let client = MyServerAppClient::new(format!("http://{}", addr));

    // Create a user
    let created = client
        .create_user(&CreateUserRequest {
            name: "Multi".to_string(),
            email: "multi@example.com".to_string(),
        })
        .await
        .expect("create should succeed");

    // Get the user
    let fetched = client
        .get_user(created.id)
        .await
        .expect("get should succeed");

    assert_eq!(fetched.id.0, created.id.0);

    // Delete the user
    client
        .delete_user(created.id)
        .await
        .expect("delete should succeed");
}

// ── Concurrent requests ─────────────────────────────────────────────────

#[tokio::test]
async fn test_client_concurrent_requests() {
    let addr = spawn_server().await;
    let client = Arc::new(MyServerAppClient::new(format!("http://{}", addr)));

    let mut handles = vec![];

    for i in 0..10 {
        let client = Arc::clone(&client);
        handles.push(tokio::spawn(async move {
            client.get_user(UserId(i)).await
        }));
    }

    for (i, handle) in handles.into_iter().enumerate() {
        let result = handle.await.expect("task should not panic");
        let response = result.expect("request should succeed");
        assert_eq!(response.id.0, i as u32);
    }
}

// ── Error type tests ────────────────────────────────────────────────────

#[tokio::test]
async fn test_client_error_display() {
    let client = MyServerAppClient::new("http://127.0.0.1:1");
    let result = client.get_user(UserId(1)).await;

    if let Err(e) = result {
        let display = format!("{}", e);
        assert!(!display.is_empty(), "Error should have display output");
    }
}

#[tokio::test]
async fn test_client_error_debug() {
    let client = MyServerAppClient::new("http://127.0.0.1:1");
    let result = client.get_user(UserId(1)).await;

    if let Err(e) = result {
        let debug = format!("{:?}", e);
        assert!(!debug.is_empty(), "Error should have debug output");
    }
}

// ── Base URL handling ───────────────────────────────────────────────────

#[tokio::test]
async fn test_client_base_url_without_trailing_slash() {
    let addr = spawn_server().await;
    let client = MyServerAppClient::new(format!("http://{}", addr));

    let result = client.get_user(UserId(1)).await;
    result.expect("should work without trailing slash");
}

#[tokio::test]
async fn test_client_base_url_string_conversion() {
    let addr = spawn_server().await;

    // Test that various string types work
    let url: String = format!("http://{}", addr);
    let client1 = MyServerAppClient::new(url.clone());
    let client2 = MyServerAppClient::new(&url);
    let client3 = MyServerAppClient::new(url.as_str());

    // All should work
    client1.get_user(UserId(1)).await.expect("String should work");
    client2.get_user(UserId(1)).await.expect("&String should work");
    client3.get_user(UserId(1)).await.expect("&str should work");
}
