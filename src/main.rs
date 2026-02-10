use crate::api::{
    albums::get_albums,
    photos::{get_next_photo, get_photos},
};
use anyhow::{Result, bail};
use axum::{Json, Router, http::StatusCode, response::Html, routing::get};
use clap::Parser;
use directories::ProjectDirs;
use std::{env, fs, path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

mod app;
use app::App;
mod api;
mod on_disk_photo;

const THIS_CRATE_NAME: &'static str = env!("CARGO_PKG_NAME");
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const AUTHOR: &'static str = env!("CARGO_PKG_AUTHORS");

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    author: &'static str,
    dedication: &'static str,
}

impl HealthResponse {
    fn new() -> Self {
        let status = "ok";
        let version = VERSION;
        let author = AUTHOR;
        let dedication = "to Janie <3";
        Self {
            status,
            version,
            author,
            dedication,
        }
    }
}

async fn health() -> (StatusCode, Json<HealthResponse>) {
    (StatusCode::OK, Json(HealthResponse::new()))
}

#[derive(Debug, Parser)]
struct Cli {
    /// Data directory. Defaults to ${XDG_DATA_HOME}/frame
    #[clap(long)]
    data_dir: Option<PathBuf>,

    /// Directory containing frontend assets. Defaults to ./dist
    #[clap(long)]
    dist_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Cli::parse();

    let Some(proj_dirs) = ProjectDirs::from("", "", THIS_CRATE_NAME) else {
        bail!("Unable to get xdg project dirs");
    };

    let data_dir = args
        .data_dir
        .unwrap_or_else(|| proj_dirs.data_dir().to_path_buf());
    if !data_dir.is_dir() {
        fs::create_dir_all(&data_dir)?;
    }

    let dist_dir = args.dist_dir.unwrap_or_else(|| {
        env::current_dir()
            .expect("Unable to get current dir")
            .join("dist")
    });
    if !dist_dir.is_dir() {
        fs::create_dir_all(&dist_dir)?;
    }

    let app = App::new(data_dir.to_path_buf()).await?;
    app.process_inbox().await?;

    let state = Arc::new(app);

    let api_photos_router = Router::new()
        .route("/next", get(get_next_photo))
        .route("/", get(get_photos));
    let api_albums_router = Router::new().route("/", get(get_albums));
    let api_router = Router::new()
        .nest("/photos", api_photos_router)
        .nest("/albums", api_albums_router);

    let admin_dist = dist_dir.join("admin");
    let index_file = admin_dist.join("index.html");
    let admin_spa = ServeDir::new(&admin_dist).not_found_service(ServeFile::new(index_file));

    let viewer_dist = dist_dir.join("viewer");
    let index_file = viewer_dist.join("index.html");
    let viewer_spa = ServeDir::new(&viewer_dist).not_found_service(ServeFile::new(index_file));

    let app = Router::new()
        .route("/_health", get(health))
        .nest("/api", api_router)
        .nest_service("/images", images_service)
        .nest_service("/admin", admin_spa)
        .fallback_service(viewer_spa)
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    info!("Serving on http://127.0.0.1:3000");
    axum::serve(listener, app).await?;

    Ok(())
}
