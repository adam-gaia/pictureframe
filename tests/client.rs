//! Integration tests for the generated AppClient.
//!
//! These tests spin up a real HTTP server and test the macro-generated client.

use pictureframe::test_helpers::{create_test_app, seed_album, seed_album_photo, seed_photo};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;

/// Spawn a test server on a random port and return the address.
async fn spawn_server() -> SocketAddr {
    let app = create_test_app().await;
    let router = app.router();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(10)).await;

    addr
}

/// Spawn a test server with seeded data and return the address.
async fn spawn_server_with_data() -> SocketAddr {
    let app = create_test_app().await;

    // Seed some test data
    let album_id = seed_album(&app, "Test Album").await;
    let photo1_id = seed_photo(&app, "hash1", "Photo 1").await;
    let photo2_id = seed_photo(&app, "hash2", "Photo 2").await;
    seed_album_photo(&app, album_id, photo1_id, 0).await;
    seed_album_photo(&app, album_id, photo2_id, 1).await;

    let router = app.router();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(10)).await;

    addr
}

// Note: The macro generates `AppClient` in the pictureframe crate.
// We need to import it to use it.
use pictureframe::app::AppClient;

#[tokio::test]
async fn test_client_get_photos_empty() {
    let addr = spawn_server().await;
    let client = AppClient::new(format!("http://{}", addr));

    let photos = client.get_photos().await.unwrap();
    assert!(photos.is_empty());
}

#[tokio::test]
async fn test_client_get_photos_with_data() {
    let addr = spawn_server_with_data().await;
    let client = AppClient::new(format!("http://{}", addr));

    let photos = client.get_photos().await.unwrap();
    assert_eq!(photos.len(), 2);
}

#[tokio::test]
async fn test_client_get_photo() {
    let addr = spawn_server_with_data().await;
    let client = AppClient::new(format!("http://{}", addr));

    // Get photos first to find a valid ID
    let photos = client.get_photos().await.unwrap();
    let photo_id = photos[0].id.0;

    let photo = client.get_photo(photo_id).await.unwrap();
    assert!(photo.title.is_some());
}

#[tokio::test]
async fn test_client_get_albums() {
    let addr = spawn_server_with_data().await;
    let client = AppClient::new(format!("http://{}", addr));

    let albums = client.get_albums().await.unwrap();
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].name, "Test Album");
}

#[tokio::test]
async fn test_client_get_album() {
    let addr = spawn_server_with_data().await;
    let client = AppClient::new(format!("http://{}", addr));

    let albums = client.get_albums().await.unwrap();
    let album_id = albums[0].id.0;

    let album = client.get_album(album_id).await.unwrap();
    assert_eq!(album.name, "Test Album");
    assert_eq!(album.photos.len(), 2);
}

#[tokio::test]
async fn test_client_get_settings() {
    let addr = spawn_server().await;
    let client = AppClient::new(format!("http://{}", addr));

    let settings = client.get_settings().await.unwrap();
    assert!(settings.current_album.is_none());
    assert_eq!(settings.interval.0, 180);
}
