use crate::app::App;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use chrono::{DateTime, NaiveDateTime};
use common::{AlbumID, RotationSettings, UpdateSettingsRequest};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub async fn get_settings(State(state): State<Arc<App>>) -> Json<RotationSettings> {
    let settings = state
        .get_settings()
        .await
        .map(|s| RotationSettings::from(s))
        .unwrap(); // TODO: handle instead of unwrap
    Json(settings)
}

pub async fn update_settings(
    State(state): State<Arc<App>>,
    Json(updates): Json<UpdateSettingsRequest>,
) {
    state.update_settings().await.unwrap(); // TODO: handle instead of unwrap
    ()
}
