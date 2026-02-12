use std::fmt::Display;

use chrono::NaiveDateTime;
use reqwasm::http::Request;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Three minutes in seconds
const THREE_MINS: u32 = 3 * 60;

#[derive(Debug, Clone, Error)]
pub enum ApiError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("HTTP {status} error: {message}")]
    Http { status: u16, message: String },
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Update<T> {
    Set(T),
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlbumID(pub i32);

impl Display for AlbumID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for AlbumID {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhotoID(pub i32);

impl Display for PhotoID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i32> for PhotoID {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
pub struct UploadPhotoRequest {
    title: Option<String>,
    artist: Option<String>,
    copyright: Option<String>,
    date_taken: Option<NaiveDateTime>,
    albums: Vec<AlbumID>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePhotoRequest {
    pub title: Option<Update<String>>,
    pub artist: Option<Update<String>>,
    pub copyright: Option<Update<String>>,
    pub date_taken: Option<Update<NaiveDateTime>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAlbumRequest {
    pub name: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAlbumRequest {
    pub name: Option<String>,
    pub notes: Option<Update<String>>,
}

/// Response struct for an album request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Album {
    pub id: AlbumID,
    pub name: String,
    pub notes: Option<String>,
    pub photos: Vec<PhotoID>,
}

/// Response struct for a photo request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub id: PhotoID,
    pub url: String,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub date_taken: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSettingsRequest {
    pub current_album_id: Option<Update<AlbumID>>,
    pub interval_seconds: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Interval(pub u32);

impl Interval {
    pub fn from_seconds(seconds: u32) -> Self {
        Self(seconds)
    }

    pub fn seconds(&self) -> u32 {
        self.0
    }
}

impl Default for Interval {
    fn default() -> Self {
        Self(THREE_MINS)
    }
}

impl From<i32> for Interval {
    fn from(seconds: i32) -> Self {
        Self(seconds as u32)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Next {
    pub photo: Photo,
    pub interval: Interval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentAlbum {
    /// Current album to serve images from
    pub album: AlbumID,
    /// Index of currently displayed image in album
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationSettings {
    pub current_album: Option<CurrentAlbum>,
    /// Number of seconds until next image
    pub interval: Interval,
}

impl Default for RotationSettings {
    fn default() -> Self {
        Self {
            current_album: None,
            interval: Interval::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
}

impl Client {
    /// Build a full URL from a relative path
    fn build_url(&self, path: impl AsRef<str>) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path.as_ref())
    }

    /// Generic GET request
    async fn get<T>(&self, path: impl AsRef<str>) -> Result<T, ApiError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path);
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        if !response.ok() {
            return Err(ApiError::Http {
                status: response.status(),
                message: response.status_text(),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| ApiError::Deserialization(e.to_string()))
    }

    /// Generic POST request
    async fn post<T, B>(&self, path: impl AsRef<str>, body: &B) -> Result<T, ApiError>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize,
    {
        let url = self.build_url(path);
        let body_json =
            serde_json::to_string(body).map_err(|e| ApiError::Serialization(e.to_string()))?;

        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        if !response.ok() {
            return Err(ApiError::Http {
                status: response.status(),
                message: response.status_text(),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| ApiError::Deserialization(e.to_string()))
    }

    /// Generic PUT request
    async fn put<T, B>(&self, path: impl AsRef<str>, body: &B) -> Result<T, ApiError>
    where
        T: for<'de> Deserialize<'de>,
        B: Serialize,
    {
        let url = self.build_url(path);
        let body_json =
            serde_json::to_string(body).map_err(|e| ApiError::Serialization(e.to_string()))?;

        let response = Request::put(&url)
            .header("Content-Type", "application/json")
            .body(body_json)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        if !response.ok() {
            return Err(ApiError::Http {
                status: response.status(),
                message: response.status_text(),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| ApiError::Deserialization(e.to_string()))
    }

    /// Generic DELETE request
    async fn delete<T>(&self, path: impl AsRef<str>) -> Result<T, ApiError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = self.build_url(path);
        let response = Request::delete(&url)
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;

        if !response.ok() {
            return Err(ApiError::Http {
                status: response.status(),
                message: response.status_text(),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| ApiError::Deserialization(e.to_string()))
    }
}

/// Public API
impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Next (for viewer)
    // ─────────────────────────────────────────────────────────────────────────

    pub async fn get_next(&self) -> Result<Next, ApiError> {
        self.get("/api/next").await
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Photos
    // ─────────────────────────────────────────────────────────────────────────

    pub async fn get_photos(&self) -> Result<Vec<Photo>, ApiError> {
        self.get("/api/photos").await
    }

    pub async fn get_photo(&self, id: PhotoID) -> Result<Photo, ApiError> {
        self.get(format!("/api/photos/{id}")).await
    }

    pub async fn update_photo(
        &self,
        id: PhotoID,
        updates: &UpdatePhotoRequest,
    ) -> Result<(), ApiError> {
        self.put(format!("/api/photos/{id}"), updates).await
    }

    pub async fn delete_photo(&self, id: PhotoID) -> Result<(), ApiError> {
        self.delete(format!("/api/photos/{id}")).await
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Albums
    // ─────────────────────────────────────────────────────────────────────────

    pub async fn get_albums(&self) -> Result<Vec<Album>, ApiError> {
        self.get("/api/albums").await
    }

    pub async fn get_album(&self, id: AlbumID) -> Result<Album, ApiError> {
        self.get(format!("/api/albums/{id}")).await
    }

    pub async fn create_album(&self, req: &CreateAlbumRequest) -> Result<Album, ApiError> {
        self.post("/api/albums", req).await
    }

    pub async fn update_album(
        &self,
        id: AlbumID,
        updates: &UpdateAlbumRequest,
    ) -> Result<(), ApiError> {
        self.put(format!("/api/albums/{id}"), updates).await
    }

    pub async fn delete_album(&self, id: AlbumID) -> Result<(), ApiError> {
        self.delete(format!("/api/albums/{id}")).await
    }

    pub async fn add_photo_to_album(
        &self,
        album_id: AlbumID,
        photo_id: PhotoID,
    ) -> Result<(), ApiError> {
        self.post(format!("/api/albums/{album_id}/photos/{photo_id}"), &()).await
    }

    pub async fn remove_photo_from_album(
        &self,
        album_id: AlbumID,
        photo_id: PhotoID,
    ) -> Result<(), ApiError> {
        self.delete(format!("/api/albums/{album_id}/photos/{photo_id}")).await
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Settings
    // ─────────────────────────────────────────────────────────────────────────

    pub async fn get_settings(&self) -> Result<RotationSettings, ApiError> {
        self.get("/api/settings").await
    }

    pub async fn update_settings(&self, updates: &UpdateSettingsRequest) -> Result<(), ApiError> {
        self.put("/api/settings", updates).await
    }
}
