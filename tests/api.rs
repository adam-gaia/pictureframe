//! Integration tests for the API endpoints.
//!
//! These tests use tower::ServiceExt::oneshot() to test the router directly
//! without spinning up an HTTP server.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{AlbumID, CreateAlbumRequest, Update, UpdateAlbumRequest, UpdatePhotoRequest, UpdateSettingsRequest};
use http_body_util::BodyExt;
use pictureframe::test_helpers::{
    create_test_app, create_test_router_with_extras, seed_album, seed_album_photo, seed_photo, set_current_album,
};
use serde_json::Value;
use tower::ServiceExt;

/// Helper to make a GET request and return the response body as JSON.
async fn get_json(router: axum::Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

    (status, json)
}

/// Helper to make a PUT request with JSON body and return the response.
async fn put_json<T: serde::Serialize>(router: axum::Router, uri: &str, body: &T) -> (StatusCode, Value) {
    let body_bytes = serde_json::to_vec(body).unwrap();
    let request = Request::builder()
        .method("PUT")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body_bytes))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

    (status, json)
}

/// Helper to make a POST request with JSON body and return the response.
async fn post_json<T: serde::Serialize>(router: axum::Router, uri: &str, body: &T) -> (StatusCode, Value) {
    let body_bytes = serde_json::to_vec(body).unwrap();
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(body_bytes))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

    (status, json)
}

/// Helper to make a DELETE request and return the response.
async fn delete(router: axum::Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

    (status, json)
}

// ─────────────────────────────────────────────────────────────────────────────
// Photos Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_photos_empty() {
    let app = create_test_app().await;
    let router = app.router();

    let (status, json) = get_json(router, "/api/photos").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::json!([]));
}

#[tokio::test]
async fn test_get_photos_with_data() {
    let app = create_test_app().await;

    // Seed test data
    seed_photo(&app, "hash1", "Photo 1").await;
    seed_photo(&app, "hash2", "Photo 2").await;

    let router = app.router();
    let (status, json) = get_json(router, "/api/photos").await;

    assert_eq!(status, StatusCode::OK);
    let photos = json.as_array().unwrap();
    assert_eq!(photos.len(), 2);

    // Check that both photos are present (order may vary with same timestamp)
    let titles: Vec<&str> = photos
        .iter()
        .map(|p| p["title"].as_str().unwrap())
        .collect();
    assert!(titles.contains(&"Photo 1"));
    assert!(titles.contains(&"Photo 2"));
}

#[tokio::test]
async fn test_get_photo_found() {
    let app = create_test_app().await;

    let photo_id = seed_photo(&app, "hash1", "Test Photo").await;

    let router = app.router();
    let (status, json) = get_json(router, &format!("/api/photos/{}", photo_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["title"], "Test Photo");
    assert_eq!(json["id"], photo_id);
}

#[tokio::test]
async fn test_get_photo_not_found() {
    let app = create_test_app().await;
    let router = app.router();

    let (status, json) = get_json(router, "/api/photos/999").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Albums Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_albums_empty() {
    let app = create_test_app().await;
    let router = app.router();

    let (status, json) = get_json(router, "/api/albums").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json, serde_json::json!([]));
}

#[tokio::test]
async fn test_get_albums_with_data() {
    let app = create_test_app().await;

    seed_album(&app, "Album A").await;
    seed_album(&app, "Album B").await;

    let router = app.router();
    let (status, json) = get_json(router, "/api/albums").await;

    assert_eq!(status, StatusCode::OK);
    let albums = json.as_array().unwrap();
    assert_eq!(albums.len(), 2);

    // Albums are ordered by name
    assert_eq!(albums[0]["name"], "Album A");
    assert_eq!(albums[1]["name"], "Album B");
}

#[tokio::test]
async fn test_get_album_found() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    let photo_id = seed_photo(&app, "hash1", "Photo 1").await;
    seed_album_photo(&app, album_id, photo_id, 0).await;

    let router = app.router();
    let (status, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "Test Album");
    assert_eq!(json["id"], album_id);

    let photos = json["photos"].as_array().unwrap();
    assert_eq!(photos.len(), 1);
    assert_eq!(photos[0], photo_id);
}

#[tokio::test]
async fn test_get_album_not_found() {
    let app = create_test_app().await;
    let router = app.router();

    let (status, json) = get_json(router, "/api/albums/999").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_settings_default() {
    let app = create_test_app().await;
    let router = app.router();

    let (status, json) = get_json(router, "/api/settings").await;

    assert_eq!(status, StatusCode::OK);
    assert!(json["current_album"].is_null());
    assert_eq!(json["interval"], 180); // Default 3 minutes
}

// ─────────────────────────────────────────────────────────────────────────────
// Next Photo Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_next_no_album_no_photos() {
    // When no album is selected and library is empty, return error
    let app = create_test_app().await;
    let router = app.router();

    let (status, json) = get_json(router, "/api/next").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("No photos"));
}

#[tokio::test]
async fn test_get_next_no_album_uses_library() {
    // When no album is selected, use entire photo library
    let app = create_test_app().await;

    // Add photos to library (but not to any album)
    seed_photo(&app, "hash1", "Library Photo 1").await;
    seed_photo(&app, "hash2", "Library Photo 2").await;

    let router = app.router();
    let (status, json) = get_json(router.clone(), "/api/next").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["photo"]["title"], "Library Photo 1");

    // Second call should return next photo
    let (status, json) = get_json(router, "/api/next").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["photo"]["title"], "Library Photo 2");
}

#[tokio::test]
async fn test_get_next_empty_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Empty Album").await;
    set_current_album(&app, album_id).await;

    let router = app.router();
    let (status, json) = get_json(router, "/api/next").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("No photos"));
}

#[tokio::test]
async fn test_get_next_returns_photo() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    let photo_id = seed_photo(&app, "hash1", "Photo 1").await;
    seed_album_photo(&app, album_id, photo_id, 0).await;
    set_current_album(&app, album_id).await;

    let router = app.router();
    let (status, json) = get_json(router, "/api/next").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["photo"]["title"], "Photo 1");
    assert_eq!(json["interval"], 180);
}

#[tokio::test]
async fn test_get_next_cycles_through_photos() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    let photo1_id = seed_photo(&app, "hash1", "Photo 1").await;
    let photo2_id = seed_photo(&app, "hash2", "Photo 2").await;
    let photo3_id = seed_photo(&app, "hash3", "Photo 3").await;
    seed_album_photo(&app, album_id, photo1_id, 0).await;
    seed_album_photo(&app, album_id, photo2_id, 1).await;
    seed_album_photo(&app, album_id, photo3_id, 2).await;
    set_current_album(&app, album_id).await;

    // First call should return Photo 1
    let router = app.clone().router();
    let (_, json) = get_json(router, "/api/next").await;
    assert_eq!(json["photo"]["title"], "Photo 1");

    // Second call should return Photo 2
    let router = app.clone().router();
    let (_, json) = get_json(router, "/api/next").await;
    assert_eq!(json["photo"]["title"], "Photo 2");

    // Third call should return Photo 3
    let router = app.clone().router();
    let (_, json) = get_json(router, "/api/next").await;
    assert_eq!(json["photo"]["title"], "Photo 3");

    // Fourth call should wrap around to Photo 1
    let router = app.clone().router();
    let (_, json) = get_json(router, "/api/next").await;
    assert_eq!(json["photo"]["title"], "Photo 1");
}

// ─────────────────────────────────────────────────────────────────────────────
// Update Settings Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_settings_set_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;

    // Set the current album via API
    let req = UpdateSettingsRequest {
        current_album_id: Some(Update::Set(AlbumID(album_id))),
        interval_seconds: None,
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, "/api/settings", &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify the album was set
    let router = app.router();
    let (_, json) = get_json(router, "/api/settings").await;
    assert_eq!(json["current_album"]["album"], album_id);
    assert_eq!(json["current_album"]["index"], 0);
}

#[tokio::test]
async fn test_update_settings_set_nonexistent_album() {
    let app = create_test_app().await;

    let req = UpdateSettingsRequest {
        current_album_id: Some(Update::Set(AlbumID(999))),
        interval_seconds: None,
    };

    let router = app.router();
    let (status, json) = put_json(router, "/api/settings", &req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_update_settings_remove_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    set_current_album(&app, album_id).await;

    // Clear the current album via API
    let req = UpdateSettingsRequest {
        current_album_id: Some(Update::Remove),
        interval_seconds: None,
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, "/api/settings", &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify the album was cleared
    let router = app.router();
    let (_, json) = get_json(router, "/api/settings").await;
    assert!(json["current_album"].is_null());
}

#[tokio::test]
async fn test_update_settings_set_interval() {
    let app = create_test_app().await;

    let req = UpdateSettingsRequest {
        current_album_id: None,
        interval_seconds: Some(60),
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, "/api/settings", &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify the interval was updated
    let router = app.router();
    let (_, json) = get_json(router, "/api/settings").await;
    assert_eq!(json["interval"], 60);
}

#[tokio::test]
async fn test_update_settings_both() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;

    // Set both album and interval
    let req = UpdateSettingsRequest {
        current_album_id: Some(Update::Set(AlbumID(album_id))),
        interval_seconds: Some(30),
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, "/api/settings", &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify both were updated
    let router = app.router();
    let (_, json) = get_json(router, "/api/settings").await;
    assert_eq!(json["current_album"]["album"], album_id);
    assert_eq!(json["interval"], 30);
}

// ─────────────────────────────────────────────────────────────────────────────
// Album CRUD Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_album() {
    let app = create_test_app().await;

    let req = CreateAlbumRequest {
        name: "My New Album".to_string(),
        notes: Some("Album notes".to_string()),
    };

    let router = app.clone().router();
    let (status, json) = post_json(router, "/api/albums", &req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "My New Album");
    assert_eq!(json["notes"], "Album notes");
    assert!(json["id"].as_i64().is_some());
    assert_eq!(json["photos"], serde_json::json!([]));

    // Verify album was persisted
    let router = app.router();
    let (status, json) = get_json(router, "/api/albums").await;
    assert_eq!(status, StatusCode::OK);
    let albums = json.as_array().unwrap();
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0]["name"], "My New Album");
}

#[tokio::test]
async fn test_create_album_without_notes() {
    let app = create_test_app().await;

    let req = CreateAlbumRequest {
        name: "Simple Album".to_string(),
        notes: None,
    };

    let router = app.router();
    let (status, json) = post_json(router, "/api/albums", &req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["name"], "Simple Album");
    assert!(json["notes"].is_null());
}

#[tokio::test]
async fn test_update_album_name() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Original Name").await;

    let req = UpdateAlbumRequest {
        name: Some("Updated Name".to_string()),
        notes: None,
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, &format!("/api/albums/{}", album_id), &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify the update
    let router = app.router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(json["name"], "Updated Name");
}

#[tokio::test]
async fn test_update_album_notes() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;

    // Set notes
    let req = UpdateAlbumRequest {
        name: None,
        notes: Some(Update::Set("New notes".to_string())),
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, &format!("/api/albums/{}", album_id), &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify notes were set
    let router = app.clone().router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(json["notes"], "New notes");

    // Remove notes
    let req = UpdateAlbumRequest {
        name: None,
        notes: Some(Update::Remove),
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, &format!("/api/albums/{}", album_id), &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify notes were removed
    let router = app.router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert!(json["notes"].is_null());
}

#[tokio::test]
async fn test_update_album_not_found() {
    let app = create_test_app().await;

    let req = UpdateAlbumRequest {
        name: Some("New Name".to_string()),
        notes: None,
    };

    let router = app.router();
    let (status, json) = put_json(router, "/api/albums/999", &req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_delete_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "To Delete").await;

    let router = app.clone().router();
    let (status, _) = delete(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(status, StatusCode::OK);

    // Verify album was deleted
    let router = app.router();
    let (status, _) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_album_clears_current_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Current Album").await;
    set_current_album(&app, album_id).await;

    // Verify it's set as current
    let router = app.clone().router();
    let (_, json) = get_json(router, "/api/settings").await;
    assert_eq!(json["current_album"]["album"], album_id);

    // Delete the album
    let router = app.clone().router();
    let (status, _) = delete(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(status, StatusCode::OK);

    // Verify current album was cleared
    let router = app.router();
    let (_, json) = get_json(router, "/api/settings").await;
    assert!(json["current_album"].is_null());
}

#[tokio::test]
async fn test_delete_album_not_found() {
    let app = create_test_app().await;

    let router = app.router();
    let (status, json) = delete(router, "/api/albums/999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Photo CRUD Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_update_photo_title() {
    let app = create_test_app().await;

    let photo_id = seed_photo(&app, "hash1", "Original Title").await;

    // Update title
    let req = UpdatePhotoRequest {
        title: Some(Update::Set("New Title".to_string())),
        artist: None,
        copyright: None,
        date_taken: None,
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, &format!("/api/photos/{}", photo_id), &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify title was updated
    let router = app.router();
    let (_, json) = get_json(router, &format!("/api/photos/{}", photo_id)).await;
    assert_eq!(json["title"], "New Title");
}

#[tokio::test]
async fn test_update_photo_remove_title() {
    let app = create_test_app().await;

    let photo_id = seed_photo(&app, "hash1", "Original Title").await;

    // Remove title
    let req = UpdatePhotoRequest {
        title: Some(Update::Remove),
        artist: None,
        copyright: None,
        date_taken: None,
    };

    let router = app.clone().router();
    let (status, _) = put_json(router, &format!("/api/photos/{}", photo_id), &req).await;
    assert_eq!(status, StatusCode::OK);

    // Verify title was removed
    let router = app.router();
    let (_, json) = get_json(router, &format!("/api/photos/{}", photo_id)).await;
    assert!(json["title"].is_null());
}

#[tokio::test]
async fn test_update_photo_not_found() {
    let app = create_test_app().await;

    let req = UpdatePhotoRequest {
        title: Some(Update::Set("New Title".to_string())),
        artist: None,
        copyright: None,
        date_taken: None,
    };

    let router = app.router();
    let (status, json) = put_json(router, "/api/photos/999", &req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_delete_photo() {
    let app = create_test_app().await;

    let photo_id = seed_photo(&app, "hash1", "To Delete").await;

    let router = app.clone().router();
    let (status, _) = delete(router, &format!("/api/photos/{}", photo_id)).await;
    assert_eq!(status, StatusCode::OK);

    // Verify photo was deleted
    let router = app.router();
    let (status, _) = get_json(router, &format!("/api/photos/{}", photo_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_photo_removes_from_albums() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    let photo_id = seed_photo(&app, "hash1", "Photo to Delete").await;
    seed_album_photo(&app, album_id, photo_id, 0).await;

    // Verify photo is in album
    let router = app.clone().router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(json["photos"].as_array().unwrap().len(), 1);

    // Delete the photo
    let router = app.clone().router();
    let (status, _) = delete(router, &format!("/api/photos/{}", photo_id)).await;
    assert_eq!(status, StatusCode::OK);

    // Verify album no longer has the photo
    let router = app.router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(json["photos"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_delete_photo_not_found() {
    let app = create_test_app().await;

    let router = app.router();
    let (status, json) = delete(router, "/api/photos/999").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Album Photo Management Tests
// ─────────────────────────────────────────────────────────────────────────────

/// Helper to make a POST request (no body) and return the response.
async fn post_empty(router: axum::Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

    (status, json)
}

#[tokio::test]
async fn test_add_photo_to_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    let photo_id = seed_photo(&app, "hash1", "Photo 1").await;

    // Verify album is empty
    let router = app.clone().router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(json["photos"].as_array().unwrap().len(), 0);

    // Add photo to album
    let router = app.clone().router();
    let (status, _) = post_empty(router, &format!("/api/albums/{}/photos/{}", album_id, photo_id)).await;
    assert_eq!(status, StatusCode::OK);

    // Verify photo was added
    let router = app.router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    let photos = json["photos"].as_array().unwrap();
    assert_eq!(photos.len(), 1);
    assert_eq!(photos[0], photo_id);
}

#[tokio::test]
async fn test_add_photo_to_album_idempotent() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    let photo_id = seed_photo(&app, "hash1", "Photo 1").await;

    // Add photo twice
    let router = app.clone().router();
    let (status, _) = post_empty(router, &format!("/api/albums/{}/photos/{}", album_id, photo_id)).await;
    assert_eq!(status, StatusCode::OK);

    let router = app.clone().router();
    let (status, _) = post_empty(router, &format!("/api/albums/{}/photos/{}", album_id, photo_id)).await;
    assert_eq!(status, StatusCode::OK);

    // Should only have one entry
    let router = app.router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(json["photos"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_add_photo_to_nonexistent_album() {
    let app = create_test_app().await;

    let photo_id = seed_photo(&app, "hash1", "Photo 1").await;

    let router = app.router();
    let (status, json) = post_empty(router, &format!("/api/albums/999/photos/{}", photo_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("Album"));
}

#[tokio::test]
async fn test_add_nonexistent_photo_to_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;

    let router = app.router();
    let (status, json) = post_empty(router, &format!("/api/albums/{}/photos/999", album_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("Photo"));
}

#[tokio::test]
async fn test_remove_photo_from_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    let photo_id = seed_photo(&app, "hash1", "Photo 1").await;
    seed_album_photo(&app, album_id, photo_id, 0).await;

    // Verify photo is in album
    let router = app.clone().router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(json["photos"].as_array().unwrap().len(), 1);

    // Remove photo from album
    let router = app.clone().router();
    let (status, _) = delete(router, &format!("/api/albums/{}/photos/{}", album_id, photo_id)).await;
    assert_eq!(status, StatusCode::OK);

    // Verify photo was removed
    let router = app.router();
    let (_, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(json["photos"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_remove_photo_from_nonexistent_album() {
    let app = create_test_app().await;

    let photo_id = seed_photo(&app, "hash1", "Photo 1").await;

    let router = app.router();
    let (status, json) = delete(router, &format!("/api/albums/999/photos/{}", photo_id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(json["error"].as_str().unwrap().contains("Album"));
}

#[tokio::test]
async fn test_remove_photo_not_in_album() {
    let app = create_test_app().await;

    let album_id = seed_album(&app, "Test Album").await;
    let photo_id = seed_photo(&app, "hash1", "Photo 1").await;

    // Should succeed even if photo not in album
    let router = app.router();
    let (status, _) = delete(router, &format!("/api/albums/{}/photos/{}", album_id, photo_id)).await;
    assert_eq!(status, StatusCode::OK);
}

// ─────────────────────────────────────────────────────────────────────────────
// Photo Upload Tests
// ─────────────────────────────────────────────────────────────────────────────

/// Helper to make a multipart POST request.
async fn post_multipart(router: axum::Router, uri: &str, field_name: &str, filename: &str, content: &[u8]) -> (StatusCode, Value) {
    let boundary = "----TestBoundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\nContent-Type: image/jpeg\r\n\r\n"
    );
    let mut body_bytes = body.into_bytes();
    body_bytes.extend_from_slice(content);
    body_bytes.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let request = Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
        .body(Body::from(body_bytes))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

    (status, json)
}

#[tokio::test]
async fn test_upload_photo_not_configured() {
    // The test app doesn't have photos_dir configured, so upload should fail
    let app = create_test_app().await;
    let router = create_test_router_with_extras(app);

    // Send a minimal JPEG-like upload
    let fake_jpeg = b"\xFF\xD8\xFF\xE0test data";
    let (status, json) = post_multipart(router, "/api/photos", "file", "test.jpg", fake_jpeg).await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(json["error"].as_str().unwrap().contains("not configured"));
}

#[tokio::test]
async fn test_upload_photo_no_file() {
    let app = create_test_app().await;
    let router = create_test_router_with_extras(app);

    // Send empty multipart request (no file field)
    let boundary = "----TestBoundary";
    let body = format!("--{boundary}--\r\n");

    let request = Request::builder()
        .method("POST")
        .uri("/api/photos")
        .header("Content-Type", format!("multipart/form-data; boundary={}", boundary))
        .body(Body::from(body))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    // Should fail because no file was provided
    // Note: The actual error depends on whether the multipart parsing fails first
    // or if the "not configured" error is returned first
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::INTERNAL_SERVER_ERROR);
}
