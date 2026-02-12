use crate::app::App;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use chrono::{DateTime, NaiveDateTime};
use common::{AlbumID, Next, Photo, PhotoID, UpdatePhotoRequest, UploadPhotoRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn get_next_photo(State(state): State<Arc<App>>) -> Json<Next> {
    let photo = state.get_next_photo().await.map(db_photo_to_photo).unwrap(); // TODO: handle and return http 5xx or whatever instead of unwrap
    let interval = state.get_interval().await;
    Json(Next { photo, interval })
}

pub async fn get_photos(State(state): State<Arc<App>>) -> Json<Vec<Photo>> {
    let db_photos = state.get_photos().await.unwrap(); // TODO: handle and return http 5xx or whatever instead of unwrap
    let photos: Vec<Photo> = db_photos.into_iter().map(db_photo_to_photo).collect();
    Json(photos)
}

pub async fn update_photo(
    State(state): State<Arc<App>>,
    Json(updates): Json<UpdatePhotoRequest>,
) -> impl IntoResponse {
    state.update_photo(updates).await.unwrap(); // TODO: handle and return http 4xx or whatever
    ()
}

pub async fn upload_photo(
    State(state): State<Arc<App>>,
    Json(photo_req): Json<UploadPhotoRequest>,
) -> impl IntoResponse {
    let db_photo = state.upload_photo(photo_req).await.unwrap(); // TODO: handle
    let photo_response = Photo::from(db_photo);
    (StatusCode::OK, Json(photo_response))
}
