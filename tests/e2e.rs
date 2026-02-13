//! End-to-end tests using real image files.
//!
//! These tests require ImageMagick to be installed and use actual JPEG files
//! from the ./data/inbox-todo directory.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use pictureframe_common::{AlbumID, CreateAlbumRequest, Update, UpdateSettingsRequest};
use http_body_util::BodyExt;
use pictureframe::App;
use pictureframe::test_helpers::create_test_router_with_extras;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

/// Path to test images directory
const TEST_IMAGES_DIR: &str = "./data/inbox-todo";

/// Check if ImageMagick is available
fn has_imagemagick() -> bool {
    which::which("magick").is_ok()
}

/// Get paths to test images
fn get_test_images() -> Vec<PathBuf> {
    let dir = std::path::Path::new(TEST_IMAGES_DIR);
    if !dir.exists() {
        return vec![];
    }

    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .map(|ext| ext.to_ascii_lowercase() == "jpg")
                .unwrap_or(false)
        })
        .take(3) // Only use 3 images for tests
        .collect()
}

/// Create an App with a real temporary data directory
async fn create_e2e_app() -> (Arc<App>, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let app = App::new(temp_dir.path().to_path_buf())
        .await
        .expect("Failed to create app");
    (Arc::new(app), temp_dir)
}

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

/// Helper to make a POST request with JSON body.
async fn post_json<T: serde::Serialize>(
    router: axum::Router,
    uri: &str,
    body: &T,
) -> (StatusCode, Value) {
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

/// Helper to make a PUT request with JSON body.
async fn put_json<T: serde::Serialize>(
    router: axum::Router,
    uri: &str,
    body: &T,
) -> (StatusCode, Value) {
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

/// Helper to make a POST request (no body).
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

/// Helper to upload a photo file via multipart form.
async fn upload_photo_file(router: axum::Router, file_path: &std::path::Path) -> (StatusCode, Value) {
    let file_bytes = std::fs::read(file_path).expect("Failed to read test image");
    let filename = file_path.file_name().unwrap().to_string_lossy();
    upload_bytes(router, filename.as_ref(), &file_bytes).await
}

/// Helper to upload raw bytes via multipart form.
async fn upload_bytes(router: axum::Router, filename: &str, file_bytes: &[u8]) -> (StatusCode, Value) {
    // Use the same multipart format as the working api.rs tests
    let boundary = "----TestBoundary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    );
    let mut body_bytes = body.into_bytes();
    body_bytes.extend_from_slice(file_bytes);
    body_bytes.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let request = Request::builder()
        .method("POST")
        .uri("/api/photos")
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={}", boundary),
        )
        .body(Body::from(body_bytes))
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);

    (status, json)
}

/// Helper to get raw bytes from a GET request.
async fn get_bytes(router: axum::Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    let status = response.status();

    let body = response.into_body().collect().await.unwrap().to_bytes();

    (status, body.to_vec())
}

// ─────────────────────────────────────────────────────────────────────────────
// End-to-End Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_e2e_upload_single_photo() {
    if !has_imagemagick() {
        eprintln!("Skipping test: ImageMagick not available");
        return;
    }

    let images = get_test_images();
    if images.is_empty() {
        eprintln!("Skipping test: No test images found in {}", TEST_IMAGES_DIR);
        return;
    }

    let (app, _temp_dir) = create_e2e_app().await;
    let router = create_test_router_with_extras(app.clone());

    // Upload a photo
    let (status, json) = upload_photo_file(router, &images[0]).await;

    assert_eq!(status, StatusCode::CREATED, "Upload failed: {:?}", json);
    assert!(json["id"].as_i64().is_some(), "Response should have id");
    assert!(json["url"].as_str().is_some(), "Response should have url");

    // Verify photo appears in list
    let router = create_test_router_with_extras(app);
    let (status, json) = get_json(router, "/api/photos").await;

    assert_eq!(status, StatusCode::OK);
    let photos = json.as_array().unwrap();
    assert_eq!(photos.len(), 1);
}

#[tokio::test]
async fn test_e2e_upload_multiple_photos() {
    if !has_imagemagick() {
        eprintln!("Skipping test: ImageMagick not available");
        return;
    }

    let images = get_test_images();
    if images.len() < 2 {
        eprintln!("Skipping test: Need at least 2 test images");
        return;
    }

    let (app, _temp_dir) = create_e2e_app().await;

    // Upload multiple photos
    for image in &images {
        let router = create_test_router_with_extras(app.clone());
        let (status, json) = upload_photo_file(router, image).await;
        assert_eq!(status, StatusCode::CREATED, "Upload failed for {:?}: {:?}", image, json);
    }

    // Verify all photos appear in list
    let router = create_test_router_with_extras(app);
    let (status, json) = get_json(router, "/api/photos").await;

    assert_eq!(status, StatusCode::OK);
    let photos = json.as_array().unwrap();
    assert_eq!(photos.len(), images.len());
}

#[tokio::test]
async fn test_e2e_full_workflow() {
    if !has_imagemagick() {
        eprintln!("Skipping test: ImageMagick not available");
        return;
    }

    let images = get_test_images();
    if images.len() < 2 {
        eprintln!("Skipping test: Need at least 2 test images");
        return;
    }

    let (app, _temp_dir) = create_e2e_app().await;

    // 1. Upload photos
    let mut photo_ids = vec![];
    for image in &images {
        let router = create_test_router_with_extras(app.clone());
        let (status, json) = upload_photo_file(router, image).await;
        assert_eq!(status, StatusCode::CREATED);
        photo_ids.push(json["id"].as_i64().unwrap() as i32);
    }

    // 2. Create an album
    let router = create_test_router_with_extras(app.clone());
    let (status, json) = post_json(
        router,
        "/api/albums",
        &CreateAlbumRequest {
            name: "Test Album".to_string(),
            notes: Some("End-to-end test album".to_string()),
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let album_id = json["id"].as_i64().unwrap() as i32;

    // 3. Add photos to album
    for photo_id in &photo_ids {
        let router = create_test_router_with_extras(app.clone());
        let (status, _) = post_empty(
            router,
            &format!("/api/albums/{}/photos/{}", album_id, photo_id),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    // 4. Verify album has photos
    let router = create_test_router_with_extras(app.clone());
    let (status, json) = get_json(router, &format!("/api/albums/{}", album_id)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["photos"].as_array().unwrap().len(), photo_ids.len());

    // 5. Set album as current
    let router = create_test_router_with_extras(app.clone());
    let (status, _) = put_json(
        router,
        "/api/settings",
        &UpdateSettingsRequest {
            current_album_id: Some(Update::Set(AlbumID(album_id))),
            interval_seconds: Some(60),
        },
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 6. Get next photo (should cycle through)
    for i in 0..photo_ids.len() {
        let router = create_test_router_with_extras(app.clone());
        let (status, json) = get_json(router, "/api/next").await;
        assert_eq!(status, StatusCode::OK, "Failed to get next photo on iteration {}", i);
        assert!(json["photo"]["id"].as_i64().is_some());
        assert_eq!(json["interval"], 60);
    }

    // 7. Verify we can serve the actual image
    let router = create_test_router_with_extras(app.clone());
    let (status, bytes) = get_bytes(router, &format!("/api/images/{}", photo_ids[0])).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!bytes.is_empty(), "Image should have content");
    // Check JPEG magic bytes
    assert_eq!(&bytes[0..2], &[0xFF, 0xD8], "Should be a valid JPEG");
}

#[tokio::test]
async fn test_e2e_image_serving() {
    if !has_imagemagick() {
        eprintln!("Skipping test: ImageMagick not available");
        return;
    }

    let images = get_test_images();
    if images.is_empty() {
        eprintln!("Skipping test: No test images found");
        return;
    }

    let (app, _temp_dir) = create_e2e_app().await;

    // Upload a photo
    let router = create_test_router_with_extras(app.clone());
    let (status, json) = upload_photo_file(router, &images[0]).await;
    assert_eq!(status, StatusCode::CREATED);
    let photo_id = json["id"].as_i64().unwrap();

    // Serve the image
    let router = create_test_router_with_extras(app);
    let (status, bytes) = get_bytes(router, &format!("/api/images/{}", photo_id)).await;

    assert_eq!(status, StatusCode::OK);
    assert!(!bytes.is_empty());
    // Verify JPEG magic bytes
    assert_eq!(&bytes[0..2], &[0xFF, 0xD8], "Should be valid JPEG");
}

#[tokio::test]
async fn test_e2e_photo_metadata_from_exif() {
    if !has_imagemagick() {
        eprintln!("Skipping test: ImageMagick not available");
        return;
    }

    let images = get_test_images();
    if images.is_empty() {
        eprintln!("Skipping test: No test images found");
        return;
    }

    let (app, _temp_dir) = create_e2e_app().await;

    // Upload a photo
    let router = create_test_router_with_extras(app.clone());
    let (status, json) = upload_photo_file(router, &images[0]).await;
    assert_eq!(status, StatusCode::CREATED);

    // The title should be set from the filename (without extension)
    let expected_title = images[0].file_stem().unwrap().to_string_lossy();
    assert_eq!(json["title"].as_str().unwrap(), expected_title.as_ref());
}

// ─────────────────────────────────────────────────────────────────────────────
// File Type Validation Tests
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_e2e_upload_rejects_png() {
    if !has_imagemagick() {
        eprintln!("Skipping test: ImageMagick not available");
        return;
    }

    let (app, _temp_dir) = create_e2e_app().await;
    let router = create_test_router_with_extras(app);

    // Try to upload a fake PNG file
    let fake_png = b"\x89PNG\r\n\x1a\nfake png data";
    let (status, json) = upload_bytes(router, "test.png", fake_png).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Only JPEG files are supported"));
}

#[tokio::test]
async fn test_e2e_upload_rejects_gif() {
    if !has_imagemagick() {
        eprintln!("Skipping test: ImageMagick not available");
        return;
    }

    let (app, _temp_dir) = create_e2e_app().await;
    let router = create_test_router_with_extras(app);

    // Try to upload a fake GIF file
    let fake_gif = b"GIF89afake gif data";
    let (status, json) = upload_bytes(router, "test.gif", fake_gif).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Only JPEG files are supported"));
}

#[tokio::test]
async fn test_e2e_upload_rejects_webp() {
    if !has_imagemagick() {
        eprintln!("Skipping test: ImageMagick not available");
        return;
    }

    let (app, _temp_dir) = create_e2e_app().await;
    let router = create_test_router_with_extras(app);

    // Try to upload a fake WebP file
    let fake_webp = b"RIFFxxxxWEBPfake webp data";
    let (status, json) = upload_bytes(router, "test.webp", fake_webp).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(json["error"]
        .as_str()
        .unwrap()
        .contains("Only JPEG files are supported"));
}
