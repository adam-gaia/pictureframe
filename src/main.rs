use anyhow::{Result, bail};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::StatusCode,
    routing::{get, post},
};
use clap::Parser;
use directories::ProjectDirs;
use pictureframe::App;
use std::{env, fs, path::PathBuf, sync::Arc};
use tokio::net::TcpListener;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

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

/// Handler for serving image files.
async fn serve_image(
    State(state): State<Arc<App>>,
    Path(id): Path<i32>,
) -> axum::response::Response {
    state.serve_image(id).await
}

/// Handler for uploading photos via multipart form.
async fn upload_photo(
    State(state): State<Arc<App>>,
    multipart: Multipart,
) -> axum::response::Response {
    state.upload_photo(multipart).await
}

#[derive(Debug, Parser)]
struct Cli {
    /// Data directory. Defaults to ${XDG_DATA_HOME}/pictureframe
    #[clap(long)]
    data_dir: Option<PathBuf>,

    /// Directory containing frontend assets. Defaults to ./dist
    #[clap(long)]
    dist_dir: Option<PathBuf>,

    /// Host address to bind to
    #[clap(long, default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[clap(long, default_value = "3000")]
    port: u16,
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
    // TODO: renable
    // app.process_inbox().await?;

    let state = Arc::new(app);

    // let api_photos_router = Router::new()
    // .route("/next", get(get_next_photo))
    // .route("/", get(get_photos));
    // let api_albums_router = Router::new().route("/", get(get_albums));
    // let api_router = Router::new()
    // .nest("/photos", api_photos_router)
    // .nest("/albums", api_albums_router);

    let admin_dist = dist_dir.join("admin");
    let admin_index = admin_dist.join("index.html");
    let admin_spa = ServeDir::new(&admin_dist).not_found_service(ServeFile::new(&admin_index));

    let viewer_dist = dist_dir.join("viewer");
    let viewer_index = viewer_dist.join("index.html");
    let viewer_spa = ServeDir::new(&viewer_dist).not_found_service(ServeFile::new(&viewer_index));

    let api_router = state.clone().router();

    // Routes for binary/multipart data (not part of the macro-generated router)
    let extra_routes = Router::new()
        .route("/api/images/{id}", get(serve_image))
        .route("/api/photos", post(upload_photo))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB limit for uploads
        .with_state(state);

    let app = Router::new()
        .route("/_health", get(health))
        .merge(api_router)
        .merge(extra_routes)
        .nest_service("/admin", admin_spa)
        .fallback_service(viewer_spa);

    let addr = format!("{}:{}", args.host, args.port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Serving on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
