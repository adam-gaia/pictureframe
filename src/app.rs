use crate::on_disk_photo::OnDiskPhoto;
use anyhow::{Result, bail};
use axum::{Json, Router, http::StatusCode, response::Html, routing::get};
use chrono::{NaiveDate, NaiveDateTime};
use clap::Parser;
use common::{Interval, RotationSettings, UpdatePhotoRequest, UploadPhotoRequest};
use directories::ProjectDirs;
use entity::prelude::*;
use entity::{album, photo};
use image::ImageDecoder;
use image_hasher::{HasherConfig, ImageHash};
use migration::{Migrator, MigratorTrait};
use sea_orm::prelude::DateTime;
use sea_orm::{ActiveValue, Database, DatabaseConnection, DbErr, EntityTrait};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::{fs, path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

const DB_FILE_NAME: &'static str = "db.sqlite";

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
    rotation_settings: RwLock<RotationSettings>,

    data_dir: PathBuf,
    magick_exec: PathBuf,
    conn: DatabaseConnection,
    inbox_dir: PathBuf,
    photos_dir: PathBuf,
    fails_dir: PathBuf,
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

        let conn = Database::connect(db_url)
            .await
            .expect("Database connection failed");
        Migrator::up(&conn, None).await?;

        let rotation_settings = RwLock::new(RotationSettings::default());

        Ok(Self {
            rotation_settings,
            data_dir,
            magick_exec,
            conn,
            inbox_dir,
            photos_dir,
            fails_dir,
        })
    }

    pub async fn get_next_photo(&self) -> Result<photo::Model> {
        let settings = self.rotation_settings.read().await;

        let album = &settings.current_album;

        todo!();
    }

    pub async fn get_interval(&self) -> Interval {
        let settings = self.rotation_settings.read().await;
        settings.interval
    }

    pub async fn upload_photo(&self, req: UploadPhotoRequest) -> Result<photo::Model> {
        // TODO
        // 1. save the photo to a file (in self.inbox_dir)
        // 2. import photo with self.import_photo() (or process entire inbox?)
        // 3. return newly created DB entry
        todo!();
    }

    pub async fn get_photos(&self) -> Result<Vec<photo::Model>> {
        let photos: Vec<photo::Model> = Photo::find().all(&self.conn).await?;
        Ok(photos)
    }

    pub async fn get_albums(&self) -> Result<Vec<Album>> {
        /*
        let albums: Vec<Album> = db_albums
            .into_iter()
            .map(|db_album| db_album_to_album(db_album, photos))
            .collect();
            */

        todo!();
    }

    async fn import_photo(&self, photo: &Path) -> Result<()> {
        info!("Processing {}", photo.display());

        let photo = OnDiskPhoto::import(photo, &self.photos_dir, &self.magick_exec).await?;
        let fullsize = photo.fullsize_path();
        let websize = photo.websize_path();
        let thumbnail = photo.thumbnail_path();

        let metadata = photo.metadata();
        let title = metadata.title();
        let artist = metadata.artist();
        let copyright = metadata.copyright();
        let date_taken = metadata.date_taken();

        let db_entry = photo::ActiveModel {
            title: ActiveValue::Set(Some(title.to_string())),

            artist: ActiveValue::Set(artist.cloned()),
            copyright: ActiveValue::Set(copyright.cloned()),
            date_taken: ActiveValue::Set(date_taken.cloned()),

            fullsize_path: ActiveValue::Set(fullsize.display().to_string()),
            websize_path: ActiveValue::Set(websize.display().to_string()),
            thumbnail_path: ActiveValue::Set(thumbnail.display().to_string()),
            ..Default::default()
        };

        debug!("Added photo entry {db_entry:?}");

        Ok(())
    }

    pub async fn update_photo(&self, updates: UpdatePhotoRequest) -> Result<()> {
        todo!();
    }

    pub async fn delete_photo(&self) -> Result<()> {
        todo!();
    }

    pub async fn process_inbox(&self) -> Result<()> {
        for entry in fs::read_dir(&self.inbox_dir)? {
            let entry = entry?;
            let path = entry.path();

            if is_photo(&path) {
                if let Err(e) = self.import_photo(&path).await {
                    move_bad_file(&path, &self.fails_dir)?;
                    bail!("Failed to process {}: {e}", path.display());
                };
            } else {
                warn!(
                    "Unable to process file {} (not a jpg). Moving to bad dir",
                    path.display()
                );
                move_bad_file(&path, &self.fails_dir)?;
            }
        }

        Ok(())
    }

    pub async fn get_settings(&self) -> Result<RotationSettings> {
        todo!();
    }

    pub async fn update_settings(&self) -> Result<()> {
        todo!();
    }
}

fn move_bad_file(path: &Path, fails_dir: &Path) -> Result<()> {
    let name = path.file_name().unwrap();
    let new_path = fails_dir.join(name);
    fs::rename(&path, new_path)?;
    Ok(())
}
