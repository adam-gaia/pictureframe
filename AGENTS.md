# AGENTS.md

## Project Overview

A self-hosted photo frame web application. A Raspberry Pi runs Firefox in kiosk mode pointed at the server. The server hosts images and serves a viewer client that cycles through album images on a timer.

## Architecture

### Stack

- **Server:** Rust, Axum, SQLite (via sqlx)
- **Clients:** Rust Leptos (compiled to WASM), served by the server
- **Database:** SQLite managed through sqlx

### Workspace Layout

```
./src/                  — Main crate: Axum server
./crates/common/        — Shared data structs (JSON serialization between client and server)
./crates/frontend-viewer/ — Leptos app: image viewer (served at /)
./crates/frontend-admin/  — Leptos app: admin panel (served at /admin)
./crates/api-client-server-macros - macros to generate axum sever handlers + api client (see ./crates/api-client-server-macros/AGENTS.md)
```

### Server State

The core application state is an `App` struct holding the database connection pool and any shared configuration. It is passed to Axum handlers as:

```rust
State(state): State<Arc<App>>
```

Settings (including global image duration) are stored in the SQLite database and read through the `App` struct.

### API Routes

| Route | Purpose |
|---|---|
| `/` | Serves the viewer Leptos WASM app |
| `/admin` | Serves the admin Leptos WASM app |
| `/api/images/<id>` | Get an image by database ID |
| `/api/next` | Get the next image to display (JSON, see below) |
| `/api/*` | CRUD routes for images, albums, settings |

### `/api/next` Response

Returns JSON with:

- Image title
- Image description
- Image URL (`/api/images/<id>`)
- Duration (how long to display the image, in seconds)

The server tracks the current position. Duration is a global setting (not per-image). Images are served sequentially. When an album is selected, photos come from that album. When no album is selected, the entire photo library is used (ordered by ID).

### Viewer Behavior

1. Call `/api/next` to get image info + duration.
2. Display the image full screen with no UI elements.
3. On image change, show a metadata overlay (title, description, date taken, etc.) that fades away after a few seconds.
4. Wait for the duration to expire, then call `/api/next` again. Loop forever.
5. On server error, display an error message (no fallback image, no retry logic — go fix the server).

The viewer has no client-side state beyond what's currently displayed. No WebSockets. Settings changes (album switch, duration change) are picked up on the next `/api/next` call after the current duration expires.

**Overlay readability:** Use a semi-transparent backdrop or text shadow/outline so metadata text is legible regardless of image brightness.

### Admin Behavior

Accessed from a separate device (not the TV). Used to:

- Set the current album
- Add/remove images
- Add/remove albums
- Change the global display duration

### Design Decisions

- Database IDs for image references (not filenames) to avoid path traversal and keep things clean.
- Set `Cache-Control` headers on `/api/images/<id>` — the viewer fetches images over a local network on a Pi, caching helps.
- Leptos is intentionally overkill for the viewer — chosen for type safety and a shared API client struct across viewer/admin/server via the `common` crate.
- The WASM bundle loads once and almost never again; bundle size is not a concern.
- Current album and current image index in the album are saved to the database. On server restart, we continue where we left off
