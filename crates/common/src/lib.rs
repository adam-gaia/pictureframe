use std::fmt::Display;

use chrono::NaiveDateTime;
use reqwasm::http::Request;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Three minutes in seconds
const THREE_MINS: u32 = 3 * 60;

/// Visual style configuration for a mat (picture frame border)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatStyle {
    pub name: String,
    pub background_color: String,
    pub padding: String,
    pub shadow: Option<String>,
    pub inner_border: Option<String>,
}

impl MatStyle {
    /// Get the mat style for a given preset name
    pub fn from_preset(name: &str) -> Self {
        match name {
            "modern" => Self {
                name: "modern".into(),
                background_color: "#ffffff".into(),
                padding: "3vmin".into(),
                shadow: Some("0 4px 20px rgba(0,0,0,0.15)".into()),
                inner_border: None,
            },
            "gallery" => Self {
                name: "gallery".into(),
                background_color: "#2c2c2c".into(),
                padding: "5vmin".into(),
                shadow: None,
                inner_border: None,
            },
            "minimal" => Self {
                name: "minimal".into(),
                background_color: "#f8f8f8".into(),
                padding: "2vmin".into(),
                shadow: None,
                inner_border: None,
            },
            "rich" => Self {
                name: "rich".into(),
                background_color: "#3a2a1a".into(),
                padding: "4vmin".into(),
                shadow: Some("inset 0 0 30px rgba(0,0,0,0.3)".into()),
                inner_border: None,
            },
            "none" => Self {
                name: "none".into(),
                background_color: "transparent".into(),
                padding: "0".into(),
                shadow: None,
                inner_border: None,
            },
            // Default: classic
            _ => Self {
                name: "classic".into(),
                background_color: "#f5f2eb".into(),
                padding: "4vmin".into(),
                shadow: None,
                inner_border: None,
            },
        }
    }

    /// Get list of all available preset names
    pub fn preset_names() -> &'static [&'static str] {
        &["classic", "modern", "gallery", "minimal", "rich", "none"]
    }
}

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
    pub mat_preset: Option<String>,
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
    pub mat_preset: String,
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
    pub mat_style: MatStyle,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mat_style_from_preset_classic() {
        let style = MatStyle::from_preset("classic");
        assert_eq!(style.name, "classic");
        assert_eq!(style.background_color, "#f5f2eb");
        assert_eq!(style.padding, "4vmin");
        assert!(style.shadow.is_none());
        assert!(style.inner_border.is_none());
    }

    #[test]
    fn test_mat_style_from_preset_modern_has_shadow() {
        let style = MatStyle::from_preset("modern");
        assert_eq!(style.name, "modern");
        assert_eq!(style.background_color, "#ffffff");
        assert_eq!(style.padding, "3vmin");
        assert!(style.shadow.is_some());
        assert!(style.shadow.unwrap().contains("rgba"));
    }

    #[test]
    fn test_mat_style_from_preset_gallery() {
        let style = MatStyle::from_preset("gallery");
        assert_eq!(style.name, "gallery");
        assert_eq!(style.background_color, "#2c2c2c");
        assert_eq!(style.padding, "5vmin");
        assert!(style.shadow.is_none());
    }

    #[test]
    fn test_mat_style_from_preset_minimal() {
        let style = MatStyle::from_preset("minimal");
        assert_eq!(style.name, "minimal");
        assert_eq!(style.background_color, "#f8f8f8");
        assert_eq!(style.padding, "2vmin");
    }

    #[test]
    fn test_mat_style_from_preset_rich_has_inset_shadow() {
        let style = MatStyle::from_preset("rich");
        assert_eq!(style.name, "rich");
        assert_eq!(style.background_color, "#3a2a1a");
        assert!(style.shadow.is_some());
        assert!(style.shadow.unwrap().contains("inset"));
    }

    #[test]
    fn test_mat_style_from_preset_none() {
        let style = MatStyle::from_preset("none");
        assert_eq!(style.name, "none");
        assert_eq!(style.background_color, "transparent");
        assert_eq!(style.padding, "0");
        assert!(style.shadow.is_none());
    }

    #[test]
    fn test_mat_style_unknown_defaults_to_classic() {
        let style = MatStyle::from_preset("unknown_preset");
        assert_eq!(style.name, "classic");
        assert_eq!(style.background_color, "#f5f2eb");
    }

    #[test]
    fn test_mat_style_empty_string_defaults_to_classic() {
        let style = MatStyle::from_preset("");
        assert_eq!(style.name, "classic");
    }

    #[test]
    fn test_preset_names_contains_all_presets() {
        let names = MatStyle::preset_names();
        assert!(names.contains(&"classic"));
        assert!(names.contains(&"modern"));
        assert!(names.contains(&"gallery"));
        assert!(names.contains(&"minimal"));
        assert!(names.contains(&"rich"));
        assert!(names.contains(&"none"));
    }

    #[test]
    fn test_preset_names_count() {
        assert_eq!(MatStyle::preset_names().len(), 6);
    }

    #[test]
    fn test_all_preset_names_produce_matching_styles() {
        for name in MatStyle::preset_names() {
            let style = MatStyle::from_preset(name);
            assert_eq!(style.name, *name, "Preset '{}' should produce style with matching name", name);
        }
    }

    #[test]
    fn test_mat_style_serialization() {
        let style = MatStyle::from_preset("modern");
        let json = serde_json::to_string(&style).unwrap();
        assert!(json.contains("\"name\":\"modern\""));
        assert!(json.contains("\"background_color\":\"#ffffff\""));
        assert!(json.contains("\"shadow\":"));
    }

    #[test]
    fn test_mat_style_deserialization() {
        let json = r##"{"name":"custom","background_color":"#000","padding":"1rem","shadow":null,"inner_border":null}"##;
        let style: MatStyle = serde_json::from_str(json).unwrap();
        assert_eq!(style.name, "custom");
        assert_eq!(style.background_color, "#000");
        assert_eq!(style.padding, "1rem");
        assert!(style.shadow.is_none());
    }
}
