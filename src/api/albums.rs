use crate::app::App;
use axum::{Json, Router, extract::State, http::StatusCode, response::Html, routing::get};
use common::{Album, PhotoID};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn get_albums(State(state): State<Arc<App>>) -> Json<Vec<Album>> {
    let albums = state.get_albums().await.unwrap(); // TODO: handle instead of unwrap
    Json(albums)
}
