use crate::models::{DbAlbum, DbPhoto, DbSettings};
use crate::on_disk_photo::OnDiskPhoto;
use anyhow::{Result, bail};
use api_macros::api;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use pictureframe_common::{
    Album, AlbumID, CreateAlbumRequest, CurrentAlbum, Interval, MatStyle, Next, Photo, PhotoID,
    RotationSettings, Update, UpdateAlbumRequest, UpdatePhotoRequest, UpdateSettingsRequest,
};
use serde::Serialize;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::{fs, path::PathBuf};
use tracing::{debug, error, info, warn};

const DB_FILE_NAME: &str = "db.sqlite";

fn move_bad_file(path: &Path, fails_dir: &Path) -> Result<()> {
    let name = path.file_name().unwrap();
    let new_path = fails_dir.join(name);
    fs::rename(path, new_path)?;
    Ok(())
}

fn db_photo_to_photo(input: &DbPhoto) -> Photo {
    Photo {
        id: PhotoID::from(input.id),
        url: format!("/api/images/{}", input.id),
        title: input.title.clone(),
        notes: input.notes.clone(),
        artist: input.artist.clone(),
        copyright: input.copyright.clone(),
        date_taken: input.date_taken,
        mat_preset: input.mat_preset.clone(),
    }
}

fn is_photo(path: &Path) -> bool {
    let Some(ext) = path.extension() else {
        return false;
    };
    let Ok(ext) = ext.to_ascii_lowercase().into_string() else {
        return false;
    };
    matches!(ext.as_str(), "jpg" | "jpeg")
}

#[derive(Debug)]
pub struct App {
    #[allow(dead_code)]
    data_dir: Option<PathBuf>,
    magick_exec: Option<PathBuf>,
    pool: SqlitePool,
    inbox_dir: Option<PathBuf>,
    photos_dir: Option<PathBuf>,
    fails_dir: Option<PathBuf>,
}

impl App {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        let Ok(magick_exec) = which::which("magick") else {
            bail!("magick: command not found")
        };

        let inbox_dir = data_dir.join("inbox");
        let photos_dir = data_dir.join("photos");
        let fails_dir = data_dir.join("fails");

        for dir in [&data_dir, &inbox_dir, &fails_dir, &photos_dir] {
            if !dir.is_dir() {
                fs::create_dir_all(dir)?;
            }
        }

        let db_path = data_dir.join(DB_FILE_NAME);
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self {
            data_dir: Some(data_dir),
            magick_exec: Some(magick_exec),
            pool,
            inbox_dir: Some(inbox_dir),
            photos_dir: Some(photos_dir),
            fails_dir: Some(fails_dir),
        })
    }

    /// Create an App from an existing database pool (for testing).
    pub fn from_pool(pool: SqlitePool) -> Self {
        Self {
            data_dir: None,
            magick_exec: None,
            pool,
            inbox_dir: None,
            photos_dir: None,
            fails_dir: None,
        }
    }

    /// Get a reference to the database pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

pub enum APIResult<T> {
    Ok(T),
    NotFound(String),
    InternalError(String),
}

impl<T: Serialize> IntoResponse for APIResult<T> {
    fn into_response(self) -> axum::response::Response {
        match self {
            APIResult::Ok(val) => (StatusCode::OK, axum::Json(val)).into_response(),
            APIResult::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "error": msg })),
            )
                .into_response(),
            APIResult::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": msg })),
            )
                .into_response(),
        }
    }
}

/// HTTP API
#[api]
impl App {
    // ─────────────────────────────────────────────────────────────────────────
    // Next Photo (for viewer)
    // ─────────────────────────────────────────────────────────────────────────

    #[api_handler(method = "GET", path = "/api/next")]
    pub async fn get_next_photo(&self) -> APIResult<Next> {
        // Get current settings from database
        let settings = match sqlx::query_as::<_, DbSettings>("SELECT * FROM settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await
        {
            Ok(s) => s,
            Err(e) => return APIResult::InternalError(format!("Failed to get settings: {}", e)),
        };

        // Get photos: from album if selected, otherwise from entire library
        let photos = if let Some(album_id) = settings.current_album_id {
            // Get photos in the current album
            match sqlx::query_as::<_, DbPhoto>(
                r#"
                SELECT p.* FROM photo p
                JOIN album_photo ap ON p.id = ap.photo_id
                WHERE ap.album_id = ?
                ORDER BY ap.position
                "#,
            )
            .bind(album_id)
            .fetch_all(&self.pool)
            .await
            {
                Ok(p) => p,
                Err(e) => return APIResult::InternalError(format!("Failed to get photos: {}", e)),
            }
        } else {
            // No album selected, use entire photo library
            match sqlx::query_as::<_, DbPhoto>("SELECT * FROM photo ORDER BY id")
                .fetch_all(&self.pool)
                .await
            {
                Ok(p) => p,
                Err(e) => return APIResult::InternalError(format!("Failed to get photos: {}", e)),
            }
        };

        if photos.is_empty() {
            return APIResult::NotFound("No photos available".to_string());
        }

        // Get current photo (with wraparound)
        let index = (settings.current_photo_index as usize) % photos.len();
        let db_photo = &photos[index];
        let photo = db_photo_to_photo(db_photo);

        // Update index for next call
        let next_index = ((index + 1) % photos.len()) as i32;
        let _ = sqlx::query("UPDATE settings SET current_photo_index = ? WHERE id = 1")
            .bind(next_index)
            .execute(&self.pool)
            .await;

        let interval = Interval::from(settings.interval_seconds);
        let mat_style = MatStyle::from_preset(&photo.mat_preset);
        APIResult::Ok(Next {
            photo,
            interval,
            mat_style,
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Photos
    // ─────────────────────────────────────────────────────────────────────────

    #[api_handler(method = "GET", path = "/api/photos")]
    pub async fn get_photos(&self) -> APIResult<Vec<Photo>> {
        match sqlx::query_as::<_, DbPhoto>("SELECT * FROM photo ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
        {
            Ok(photos) => APIResult::Ok(photos.iter().map(db_photo_to_photo).collect()),
            Err(e) => APIResult::InternalError(format!("Failed to get photos: {}", e)),
        }
    }

    #[api_handler(method = "GET", path = "/api/photos/{id}")]
    pub async fn get_photo(&self, #[path] id: i32) -> APIResult<Photo> {
        match sqlx::query_as::<_, DbPhoto>("SELECT * FROM photo WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(photo)) => APIResult::Ok(db_photo_to_photo(&photo)),
            Ok(None) => APIResult::NotFound(format!("Photo {} not found", id)),
            Err(e) => APIResult::InternalError(format!("Failed to get photo: {}", e)),
        }
    }

    #[api_handler(method = "PUT", path = "/api/photos/{id}")]
    pub async fn update_photo(
        &self,
        #[path] id: i32,
        #[body] req: UpdatePhotoRequest,
    ) -> APIResult<()> {
        // Check photo exists
        let exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM photo WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await;

        match exists {
            Ok(Some(_)) => {}
            Ok(None) => return APIResult::NotFound(format!("Photo {} not found", id)),
            Err(e) => return APIResult::InternalError(format!("Database error: {}", e)),
        }

        // Update title if provided
        if let Some(title_update) = &req.title {
            match title_update {
                Update::Set(title) => {
                    if let Err(e) = sqlx::query(
                        "UPDATE photo SET title = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    )
                    .bind(title)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    {
                        return APIResult::InternalError(format!("Failed to update photo: {}", e));
                    }
                }
                Update::Remove => {
                    if let Err(e) = sqlx::query("UPDATE photo SET title = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(id)
                        .execute(&self.pool)
                        .await
                    {
                        return APIResult::InternalError(format!("Failed to update photo: {}", e));
                    }
                }
            }
        }

        // Update artist if provided
        if let Some(artist_update) = &req.artist {
            match artist_update {
                Update::Set(artist) => {
                    if let Err(e) = sqlx::query(
                        "UPDATE photo SET artist = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    )
                    .bind(artist)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    {
                        return APIResult::InternalError(format!("Failed to update photo: {}", e));
                    }
                }
                Update::Remove => {
                    if let Err(e) = sqlx::query("UPDATE photo SET artist = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(id)
                        .execute(&self.pool)
                        .await
                    {
                        return APIResult::InternalError(format!("Failed to update photo: {}", e));
                    }
                }
            }
        }

        // Update copyright if provided
        if let Some(copyright_update) = &req.copyright {
            match copyright_update {
                Update::Set(copyright) => {
                    if let Err(e) = sqlx::query("UPDATE photo SET copyright = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(copyright)
                        .bind(id)
                        .execute(&self.pool)
                        .await
                    {
                        return APIResult::InternalError(format!("Failed to update photo: {}", e));
                    }
                }
                Update::Remove => {
                    if let Err(e) = sqlx::query("UPDATE photo SET copyright = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(id)
                        .execute(&self.pool)
                        .await
                    {
                        return APIResult::InternalError(format!("Failed to update photo: {}", e));
                    }
                }
            }
        }

        // Update date_taken if provided
        if let Some(date_update) = &req.date_taken {
            match date_update {
                Update::Set(date) => {
                    if let Err(e) = sqlx::query("UPDATE photo SET date_taken = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(date)
                        .bind(id)
                        .execute(&self.pool)
                        .await
                    {
                        return APIResult::InternalError(format!("Failed to update photo: {}", e));
                    }
                }
                Update::Remove => {
                    if let Err(e) = sqlx::query("UPDATE photo SET date_taken = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(id)
                        .execute(&self.pool)
                        .await
                    {
                        return APIResult::InternalError(format!("Failed to update photo: {}", e));
                    }
                }
            }
        }

        // Update mat_preset if provided
        if let Some(preset) = &req.mat_preset {
            // Validate preset exists
            if !MatStyle::preset_names().contains(&preset.as_str()) {
                return APIResult::InternalError(format!("Unknown mat preset: {}", preset));
            }
            if let Err(e) = sqlx::query(
                "UPDATE photo SET mat_preset = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(preset)
            .bind(id)
            .execute(&self.pool)
            .await
            {
                return APIResult::InternalError(format!("Failed to update photo: {}", e));
            }
        }

        APIResult::Ok(())
    }

    #[api_handler(method = "DELETE", path = "/api/photos/{id}")]
    pub async fn delete_photo(&self, #[path] id: i32) -> APIResult<()> {
        // Check photo exists
        let photo = match sqlx::query_as::<_, DbPhoto>("SELECT * FROM photo WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(p)) => p,
            Ok(None) => return APIResult::NotFound(format!("Photo {} not found", id)),
            Err(e) => return APIResult::InternalError(format!("Database error: {}", e)),
        };

        // Delete album-photo associations first
        if let Err(e) = sqlx::query("DELETE FROM album_photo WHERE photo_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
        {
            return APIResult::InternalError(format!("Failed to delete photo from albums: {}", e));
        }

        // Delete the database record
        if let Err(e) = sqlx::query("DELETE FROM photo WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
        {
            return APIResult::InternalError(format!("Failed to delete photo: {}", e));
        }

        // Try to delete the image files (best effort, don't fail if files don't exist)
        if let Some(photos_dir) = &self.photos_dir {
            let paths = [
                std::path::Path::new(&photo.fullsize_path),
                std::path::Path::new(&photo.websize_path),
                std::path::Path::new(&photo.thumbnail_path),
            ];
            for path in paths {
                if path.starts_with(photos_dir) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }

        APIResult::Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Albums
    // ─────────────────────────────────────────────────────────────────────────

    #[api_handler(method = "GET", path = "/api/albums")]
    pub async fn get_albums(&self) -> APIResult<Vec<Album>> {
        let db_albums = match sqlx::query_as::<_, DbAlbum>("SELECT * FROM album ORDER BY name")
            .fetch_all(&self.pool)
            .await
        {
            Ok(albums) => albums,
            Err(e) => return APIResult::InternalError(format!("Failed to get albums: {}", e)),
        };

        let mut albums = Vec::new();
        for db_album in db_albums {
            let photo_ids = match sqlx::query_scalar::<_, i32>(
                "SELECT photo_id FROM album_photo WHERE album_id = ? ORDER BY position",
            )
            .bind(db_album.id)
            .fetch_all(&self.pool)
            .await
            {
                Ok(ids) => ids.into_iter().map(PhotoID::from).collect(),
                Err(e) => {
                    return APIResult::InternalError(format!("Failed to get album photos: {}", e));
                }
            };

            albums.push(Album {
                id: AlbumID::from(db_album.id),
                name: db_album.name,
                notes: db_album.notes,
                photos: photo_ids,
            });
        }

        APIResult::Ok(albums)
    }

    #[api_handler(method = "GET", path = "/api/albums/{id}")]
    pub async fn get_album(&self, #[path] id: i32) -> APIResult<Album> {
        let db_album = match sqlx::query_as::<_, DbAlbum>("SELECT * FROM album WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(album)) => album,
            Ok(None) => return APIResult::NotFound(format!("Album {} not found", id)),
            Err(e) => return APIResult::InternalError(format!("Failed to get album: {}", e)),
        };

        let photo_ids = match sqlx::query_scalar::<_, i32>(
            "SELECT photo_id FROM album_photo WHERE album_id = ? ORDER BY position",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        {
            Ok(ids) => ids.into_iter().map(PhotoID::from).collect(),
            Err(e) => {
                return APIResult::InternalError(format!("Failed to get album photos: {}", e));
            }
        };

        APIResult::Ok(Album {
            id: AlbumID::from(db_album.id),
            name: db_album.name,
            notes: db_album.notes,
            photos: photo_ids,
        })
    }

    #[api_handler(method = "POST", path = "/api/albums")]
    pub async fn create_album(&self, #[body] req: CreateAlbumRequest) -> APIResult<Album> {
        if req.name.trim().is_empty() {
            return APIResult::InternalError("Album name cannot be empty".to_string());
        }

        let result = sqlx::query_scalar::<_, i32>(
            "INSERT INTO album (name, notes) VALUES (?, ?) RETURNING id",
        )
        .bind(&req.name)
        .bind(&req.notes)
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(id) => APIResult::Ok(Album {
                id: AlbumID::from(id),
                name: req.name,
                notes: req.notes,
                photos: vec![],
            }),
            Err(e) => APIResult::InternalError(format!("Failed to create album: {}", e)),
        }
    }

    #[api_handler(method = "PUT", path = "/api/albums/{id}")]
    pub async fn update_album(
        &self,
        #[path] id: i32,
        #[body] req: UpdateAlbumRequest,
    ) -> APIResult<()> {
        // Check album exists
        let exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM album WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await;

        match exists {
            Ok(Some(_)) => {}
            Ok(None) => return APIResult::NotFound(format!("Album {} not found", id)),
            Err(e) => return APIResult::InternalError(format!("Database error: {}", e)),
        }

        // Update name if provided
        if let Some(name) = &req.name {
            if name.trim().is_empty() {
                return APIResult::InternalError("Album name cannot be empty".to_string());
            }
            if let Err(e) = sqlx::query(
                "UPDATE album SET name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(name)
            .bind(id)
            .execute(&self.pool)
            .await
            {
                return APIResult::InternalError(format!("Failed to update album: {}", e));
            }
        }

        // Update notes if provided
        if let Some(notes_update) = &req.notes {
            match notes_update {
                Update::Set(notes) => {
                    if let Err(e) = sqlx::query(
                        "UPDATE album SET notes = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                    )
                    .bind(notes)
                    .bind(id)
                    .execute(&self.pool)
                    .await
                    {
                        return APIResult::InternalError(format!("Failed to update album: {}", e));
                    }
                }
                Update::Remove => {
                    if let Err(e) = sqlx::query("UPDATE album SET notes = NULL, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(id)
                        .execute(&self.pool)
                        .await
                    {
                        return APIResult::InternalError(format!("Failed to update album: {}", e));
                    }
                }
            }
        }

        APIResult::Ok(())
    }

    #[api_handler(method = "DELETE", path = "/api/albums/{id}")]
    pub async fn delete_album(&self, #[path] id: i32) -> APIResult<()> {
        // Check album exists
        let exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM album WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await;

        match exists {
            Ok(Some(_)) => {}
            Ok(None) => return APIResult::NotFound(format!("Album {} not found", id)),
            Err(e) => return APIResult::InternalError(format!("Database error: {}", e)),
        }

        // Delete album-photo associations first
        if let Err(e) = sqlx::query("DELETE FROM album_photo WHERE album_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
        {
            return APIResult::InternalError(format!("Failed to delete album photos: {}", e));
        }

        // If this album is the current album, clear it
        let _ = sqlx::query("UPDATE settings SET current_album_id = NULL, current_photo_index = 0 WHERE current_album_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await;

        // Delete the album
        if let Err(e) = sqlx::query("DELETE FROM album WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
        {
            return APIResult::InternalError(format!("Failed to delete album: {}", e));
        }

        APIResult::Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Album Photos (Managing photo membership in albums)
    // ─────────────────────────────────────────────────────────────────────────

    #[api_handler(method = "POST", path = "/api/albums/{album_id}/photos/{photo_id}")]
    pub async fn add_photo_to_album(
        &self,
        #[path] album_id: i32,
        #[path] photo_id: i32,
    ) -> APIResult<()> {
        // Check album exists
        let album_exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM album WHERE id = ?")
            .bind(album_id)
            .fetch_optional(&self.pool)
            .await;

        match album_exists {
            Ok(Some(_)) => {}
            Ok(None) => return APIResult::NotFound(format!("Album {} not found", album_id)),
            Err(e) => return APIResult::InternalError(format!("Database error: {}", e)),
        }

        // Check photo exists
        let photo_exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM photo WHERE id = ?")
            .bind(photo_id)
            .fetch_optional(&self.pool)
            .await;

        match photo_exists {
            Ok(Some(_)) => {}
            Ok(None) => return APIResult::NotFound(format!("Photo {} not found", photo_id)),
            Err(e) => return APIResult::InternalError(format!("Database error: {}", e)),
        }

        // Check if photo is already in album
        let already_in_album = sqlx::query_scalar::<_, i32>(
            "SELECT 1 FROM album_photo WHERE album_id = ? AND photo_id = ?",
        )
        .bind(album_id)
        .bind(photo_id)
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(_)) = already_in_album {
            // Photo already in album, nothing to do
            return APIResult::Ok(());
        }

        // Get the next position
        let max_position = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(position) FROM album_photo WHERE album_id = ?",
        )
        .bind(album_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(None)
        .unwrap_or(-1);

        // Add photo to album
        if let Err(e) =
            sqlx::query("INSERT INTO album_photo (album_id, photo_id, position) VALUES (?, ?, ?)")
                .bind(album_id)
                .bind(photo_id)
                .bind(max_position + 1)
                .execute(&self.pool)
                .await
        {
            return APIResult::InternalError(format!("Failed to add photo to album: {}", e));
        }

        APIResult::Ok(())
    }

    #[api_handler(method = "DELETE", path = "/api/albums/{album_id}/photos/{photo_id}")]
    pub async fn remove_photo_from_album(
        &self,
        #[path] album_id: i32,
        #[path] photo_id: i32,
    ) -> APIResult<()> {
        // Check album exists
        let album_exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM album WHERE id = ?")
            .bind(album_id)
            .fetch_optional(&self.pool)
            .await;

        match album_exists {
            Ok(Some(_)) => {}
            Ok(None) => return APIResult::NotFound(format!("Album {} not found", album_id)),
            Err(e) => return APIResult::InternalError(format!("Database error: {}", e)),
        }

        // Remove photo from album (don't fail if it wasn't there)
        if let Err(e) = sqlx::query("DELETE FROM album_photo WHERE album_id = ? AND photo_id = ?")
            .bind(album_id)
            .bind(photo_id)
            .execute(&self.pool)
            .await
        {
            return APIResult::InternalError(format!("Failed to remove photo from album: {}", e));
        }

        APIResult::Ok(())
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Mat Presets
    // ─────────────────────────────────────────────────────────────────────────

    #[api_handler(method = "GET", path = "/api/mat-presets")]
    pub async fn get_mat_presets(&self) -> APIResult<Vec<MatStyle>> {
        let presets = MatStyle::preset_names()
            .iter()
            .map(|name| MatStyle::from_preset(name))
            .collect();
        APIResult::Ok(presets)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Settings
    // ─────────────────────────────────────────────────────────────────────────

    #[api_handler(method = "GET", path = "/api/settings")]
    pub async fn get_settings(&self) -> APIResult<RotationSettings> {
        let settings = match sqlx::query_as::<_, DbSettings>("SELECT * FROM settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await
        {
            Ok(s) => s,
            Err(e) => return APIResult::InternalError(format!("Failed to get settings: {}", e)),
        };

        let current_album = match settings.current_album_id {
            Some(album_id) => Some(CurrentAlbum {
                album: AlbumID::from(album_id),
                index: settings.current_photo_index as usize,
            }),
            None => None,
        };

        APIResult::Ok(RotationSettings {
            current_album,
            interval: Interval::from(settings.interval_seconds),
        })
    }

    #[api_handler(method = "PUT", path = "/api/settings")]
    pub async fn update_settings(&self, #[body] req: UpdateSettingsRequest) -> APIResult<()> {
        // Handle current_album_id update
        if let Some(album_update) = req.current_album_id {
            match album_update {
                Update::Set(album_id) => {
                    // Verify album exists
                    let exists = sqlx::query_scalar::<_, i32>("SELECT 1 FROM album WHERE id = ?")
                        .bind(album_id.0)
                        .fetch_optional(&self.pool)
                        .await;

                    match exists {
                        Ok(Some(_)) => {
                            // Album exists, update settings and reset index
                            if let Err(e) = sqlx::query(
                                "UPDATE settings SET current_album_id = ?, current_photo_index = 0 WHERE id = 1",
                            )
                            .bind(album_id.0)
                            .execute(&self.pool)
                            .await
                            {
                                return APIResult::InternalError(format!(
                                    "Failed to update settings: {}",
                                    e
                                ));
                            }
                        }
                        Ok(None) => {
                            return APIResult::NotFound(format!("Album {} not found", album_id.0));
                        }
                        Err(e) => {
                            return APIResult::InternalError(format!("Database error: {}", e));
                        }
                    }
                }
                Update::Remove => {
                    // Clear current album
                    if let Err(e) = sqlx::query(
                        "UPDATE settings SET current_album_id = NULL, current_photo_index = 0 WHERE id = 1",
                    )
                    .execute(&self.pool)
                    .await
                    {
                        return APIResult::InternalError(format!(
                            "Failed to update settings: {}",
                            e
                        ));
                    }
                }
            }
        }

        // Handle interval_seconds update
        if let Some(interval) = req.interval_seconds {
            if interval < 1 {
                return APIResult::InternalError("Interval must be at least 1 second".to_string());
            }
            if let Err(e) = sqlx::query("UPDATE settings SET interval_seconds = ? WHERE id = 1")
                .bind(interval)
                .execute(&self.pool)
                .await
            {
                return APIResult::InternalError(format!("Failed to update settings: {}", e));
            }
        }

        APIResult::Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Non-macro handlers (raw Axum handlers for binary/multipart data)
// ─────────────────────────────────────────────────────────────────────────────

impl App {
    /// Upload a new photo via multipart form data.
    /// This is a raw Axum handler because it needs multipart form handling.
    pub async fn upload_photo(
        &self,
        mut multipart: axum::extract::Multipart,
    ) -> axum::response::Response {
        use axum::body::Body;
        use axum::http::{Response, StatusCode};

        // Check if we have the required directories configured
        let Some(photos_dir) = &self.photos_dir else {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"error": "Server not configured for uploads"}"#,
                ))
                .unwrap();
        };
        let Some(magick_exec) = &self.magick_exec else {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "ImageMagick not available"}"#))
                .unwrap();
        };

        // Extract the file from multipart form
        let mut file_data: Option<(String, Vec<u8>)> = None;

        while let Ok(Some(field)) = multipart.next_field().await {
            let name = field.name().unwrap_or("").to_string();

            if name == "file" || name == "photo" {
                let filename = field.file_name().unwrap_or("upload.jpg").to_string();
                match field.bytes().await {
                    Ok(bytes) => {
                        file_data = Some((filename, bytes.to_vec()));
                        break;
                    }
                    Err(e) => {
                        return Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .header("Content-Type", "application/json")
                            .body(Body::from(format!(
                                r#"{{"error": "Failed to read file: {}"}}"#,
                                e
                            )))
                            .unwrap();
                    }
                }
            }
        }

        let Some((filename, bytes)) = file_data else {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    r#"{"error": "No file provided. Include a 'file' or 'photo' field."}"#,
                ))
                .unwrap();
        };

        // Validate file extension
        let ext = std::path::Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !matches!(ext.as_str(), "jpg" | "jpeg") {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .body(Body::from(r#"{"error": "Only JPEG files are supported"}"#))
                .unwrap();
        }

        // Save to a temp file
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"error": "Failed to create temp directory: {}"}}"#,
                        e
                    )))
                    .unwrap();
            }
        };
        let temp_path = temp_dir.path().join(&filename);

        if let Err(e) = std::fs::write(&temp_path, &bytes) {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"error": "Failed to write temp file: {}"}}"#,
                    e
                )))
                .unwrap();
        }

        // Import the photo using OnDiskPhoto
        let photo = match OnDiskPhoto::import(&temp_path, photos_dir, magick_exec).await {
            Ok(p) => p,
            Err(e) => {
                error!("{e}");
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"error": "Failed to process photo: {}"}}"#,
                        e
                    )))
                    .unwrap();
            }
        };

        // Insert into database
        let metadata = photo.metadata();
        let result = sqlx::query_scalar::<_, i32>(
            r#"
            INSERT INTO photo (hash, title, artist, copyright, date_taken, fullsize_path, websize_path, thumbnail_path)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING id
            "#,
        )
        .bind(photo.hash())
        .bind(metadata.title())
        .bind(metadata.artist())
        .bind(metadata.copyright())
        .bind(metadata.date_taken())
        .bind(photo.fullsize_path().display().to_string())
        .bind(photo.websize_path().display().to_string())
        .bind(photo.thumbnail_path().display().to_string())
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(id) => {
                let response_photo = Photo {
                    id: PhotoID::from(id),
                    url: format!("/api/images/{}", id),
                    title: Some(metadata.title().clone()),
                    notes: None,
                    artist: metadata.artist().cloned(),
                    copyright: metadata.copyright().cloned(),
                    date_taken: metadata.date_taken().cloned(),
                    mat_preset: "classic".to_string(),
                };

                let json = serde_json::to_string(&response_photo).unwrap();
                Response::builder()
                    .status(StatusCode::CREATED)
                    .header("Content-Type", "application/json")
                    .body(Body::from(json))
                    .unwrap()
            }
            Err(e) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(format!(
                    r#"{{"error": "Failed to save photo: {}"}}"#,
                    e
                )))
                .unwrap(),
        }
    }

    /// Serve the websize image for a photo.
    /// This is a raw Axum handler (not using the api macro) because it returns binary data.
    pub async fn serve_image(&self, id: i32) -> axum::response::Response {
        use axum::body::Body;
        use axum::http::{Response, StatusCode, header};

        // Look up the photo
        let photo = match sqlx::query_as::<_, DbPhoto>("SELECT * FROM photo WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
        {
            Ok(Some(p)) => p,
            Ok(None) => {
                return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Photo not found"))
                    .unwrap();
            }
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Database error: {}", e)))
                    .unwrap();
            }
        };

        // Read the websize image file
        let path = std::path::Path::new(&photo.websize_path);
        match tokio::fs::read(path).await {
            Ok(bytes) => {
                // Determine content type from extension
                let content_type = match path.extension().and_then(|e| e.to_str()) {
                    Some("jpg") | Some("jpeg") => "image/jpeg",
                    Some("png") => "image/png",
                    Some("gif") => "image/gif",
                    Some("webp") => "image/webp",
                    _ => "application/octet-stream",
                };

                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, content_type)
                    .header(header::CACHE_CONTROL, "public, max-age=86400") // Cache for 1 day
                    .body(Body::from(bytes))
                    .unwrap()
            }
            Err(e) => {
                warn!("Failed to read image file {}: {}", path.display(), e);
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("Image file not found"))
                    .unwrap()
            }
        }
    }

    /// Internal method to import a photo from disk
    #[allow(dead_code)]
    async fn import_photo(&self, photo_path: &Path) -> Result<()> {
        info!("Processing {}", photo_path.display());

        let photos_dir = self.photos_dir.as_ref().expect("photos_dir not configured");
        let magick_exec = self
            .magick_exec
            .as_ref()
            .expect("magick_exec not configured");
        let photo = OnDiskPhoto::import(photo_path, photos_dir, magick_exec).await?;
        let fullsize = photo.fullsize_path();
        let websize = photo.websize_path();
        let thumbnail = photo.thumbnail_path();

        let metadata = photo.metadata();
        let title = metadata.title();
        let artist = metadata.artist();
        let copyright = metadata.copyright();
        let date_taken = metadata.date_taken();
        let hash = photo.hash();

        sqlx::query(
            r#"
            INSERT INTO photo (hash, title, artist, copyright, date_taken, fullsize_path, websize_path, thumbnail_path)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(hash)
        .bind(title)
        .bind(artist)
        .bind(copyright)
        .bind(date_taken)
        .bind(fullsize.display().to_string())
        .bind(websize.display().to_string())
        .bind(thumbnail.display().to_string())
        .execute(&self.pool)
        .await?;

        debug!("Added photo with hash {}", hash);

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn process_inbox(&self) -> Result<()> {
        let inbox_dir = self.inbox_dir.as_ref().expect("inbox_dir not configured");
        let fails_dir = self.fails_dir.as_ref().expect("fails_dir not configured");

        for entry in fs::read_dir(inbox_dir)? {
            let entry = entry?;
            let path = entry.path();

            if is_photo(&path) {
                if let Err(e) = self.import_photo(&path).await {
                    move_bad_file(&path, fails_dir)?;
                    bail!("Failed to process {}: {e}", path.display());
                };
            } else {
                warn!(
                    "Unable to process file {} (not a jpg). Moving to bad dir",
                    path.display()
                );
                move_bad_file(&path, fails_dir)?;
            }
        }

        Ok(())
    }
}
