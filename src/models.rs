use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Database model for a photo
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbPhoto {
    pub id: i32,
    pub hash: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub notes: Option<String>,
    pub date_taken: Option<NaiveDateTime>,
    pub fullsize_path: String,
    pub websize_path: String,
    pub thumbnail_path: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Database model for an album
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DbAlbum {
    pub id: i32,
    pub name: String,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Database model for album-photo junction
#[derive(Debug, Clone, FromRow)]
pub struct DbAlbumPhoto {
    pub album_id: i32,
    pub photo_id: i32,
    pub position: i32,
    pub created_at: NaiveDateTime,
}

/// Database model for app settings
#[derive(Debug, Clone, FromRow)]
pub struct DbSettings {
    pub id: i32,
    pub current_album_id: Option<i32>,
    pub current_photo_index: i32,
    pub interval_seconds: i32,
    pub updated_at: NaiveDateTime,
}

/// For inserting a new photo
#[derive(Debug)]
pub struct NewPhoto {
    pub hash: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub notes: Option<String>,
    pub date_taken: Option<NaiveDateTime>,
    pub fullsize_path: String,
    pub websize_path: String,
    pub thumbnail_path: String,
}

/// For inserting a new album
#[derive(Debug)]
pub struct NewAlbum {
    pub name: String,
    pub notes: Option<String>,
}
