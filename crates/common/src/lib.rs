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

#[derive(Debug, Serialize, Deserialize)]
pub struct AlbumID(u32);

impl Display for AlbumID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PhotoID(u32);

impl Display for PhotoID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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
    title: Option<Update<String>>,
    artist: Option<Update<String>>,
    copyright: Option<Update<String>>,
    date_taken: Option<Update<NaiveDateTime>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAlbumRequest {}

/// Response struct for an album request
#[derive(Debug, Serialize, Deserialize)]
pub struct Album {
    pub name: String,
    pub notes: Option<String>,
    pub photos: Vec<PhotoID>,
}

/// Response struct for an photo request
#[derive(Debug, Serialize, Deserialize)]
pub struct Photo {
    pub title: Option<String>,
    pub notes: Option<String>,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub date_taken: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSettingsRequest {
    current_album: Option<Update<Album>>,
    rotation_seconds: Option<Update<usize>>,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub struct Interval(u32);

impl Default for Interval {
    fn default() -> Self {
        Self(THREE_MINS)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Next {
    pub photo: Photo,
    pub interval: Interval,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CurrentAlbum {
    /// Current album to serve images from
    pub album: AlbumID,
    /// Index of currently displayed image in album
    pub index: usize,
}

#[derive(Debug, Serialize, Deserialize)]
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

    pub async fn get_next_image(&self) -> Result<Photo, ApiError> {
        self.get("/api/photos/next").await
    }

    pub async fn get_settings(&self) -> Result<RotationSettings, ApiError> {
        self.get("/api/settings").await
    }

    pub async fn update_settings(&self, updates: &UpdateSettingsRequest) -> Result<(), ApiError> {
        self.put("/api/settings", updates).await
    }

    pub async fn update_photo(
        &self,
        id: PhotoID,
        updates: &UpdatePhotoRequest,
    ) -> Result<(), ApiError> {
        self.put(format!("/api/photos/{id}"), updates).await
    }

    pub async fn update_album(
        &self,
        id: AlbumID,
        updates: &UpdateAlbumRequest,
    ) -> Result<(), ApiError> {
        self.put(format!("/api/albums/{id}"), updates).await
    }
}
