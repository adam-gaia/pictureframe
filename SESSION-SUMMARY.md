# Session Summary: SQLx + API Macro Refactor + Query Params

## Completed Work

### Phase 1: SQLx Migration (Complete)
- Replaced SeaORM with SQLx for database access
- Created `migrations/` directory with 4 SQL migration files
- Created `src/models.rs` with `FromRow` database models

### Phase 2 & 3: API Handlers (Complete)
- Implemented all CRUD endpoints for photos, albums, and settings
- Implemented `/api/next` for photo rotation

### Phase 4: Test Suite (Complete)
- Created test infrastructure with comprehensive coverage
- 42 API integration tests
- 6 client tests

### Phase 5: Photo Upload (Complete)
- Implemented `POST /api/photos` with multipart form handling
- Added `multipart` feature to axum
- Validates JPEG files, processes via ImageMagick, extracts EXIF metadata

### Phase 6: End-to-End Tests (Complete)
- Created `tests/e2e.rs` with 5 tests using real images from `./data/inbox-todo/`
- Tests require ImageMagick and skip gracefully if unavailable

### Phase 7: Frontend Integration (Complete)
- Fixed and updated the Leptos viewer frontend
- Implemented full admin panel with Photos, Albums, and Settings tabs
- Added missing client methods for admin functionality

### Phase 8: Query Parameter Support (Complete)
- Added `#[query]` attribute for query string parameters in API handlers
- Server generates `axum::extract::Query<T>` extractor
- Client serializes query struct to URL query string using `serde_urlencoded`

### Phase 9: Admin Photo Upload (Complete - This Session)
- Added "Upload Photo" button to the Photos tab in admin panel
- Uses web-sys FormData and fetch API for multipart file upload
- Shows upload progress and error messages

---

## This Session's Work

### Query Parameter Support (`crates/api-client-server-macros/`)

Added `#[query]` parameter support to the API macro, enabling query string parameters for endpoints.

**Usage:**
```rust
#[api_handler(method = "GET", path = "/users")]
pub async fn list_users(&self, #[query] params: ListUsersParams) -> MyResult<Vec<User>> {
    // params.page, params.limit, etc.
}
```

**Implementation:**
- Added `ParamKind::Query` variant
- Server: Generates `axum::extract::Query<T>` extractor
- Client: Serializes query struct to URL query string using `serde_urlencoded`
- Query struct must implement `Serialize + Deserialize`

**Tests Added:**
- 10 new unit tests for query param parsing/generation
- 5 new router integration tests (query params in HTTP requests)
- 4 new client integration tests (query params serialization)
- 1 new macro expansion test

**Documentation:**
- Updated AGENTS.md with query param examples and requirements
- Updated error messages to include `#[query]` option

### Admin Photo Upload (`crates/frontend-admin/`)

Added photo upload functionality to the admin panel Photos tab.

**Implementation:**
- Hidden file input triggered by "Upload Photo" button
- Uses web-sys `FormData` and `fetch` API for multipart upload
- Uploads to `POST /api/photos` endpoint
- Shows "Uploading..." state during upload
- Displays error message on failure
- Auto-refreshes photo list on success

**Dependencies Added:**
- `wasm-bindgen` and `wasm-bindgen-futures`
- `web-sys` with features: File, FileList, FormData, HtmlInputElement, Request, RequestInit, Response, Window

---

## Current State

**All tests passing:**

Main project (53 tests):
```
tests/api.rs    - 42 tests (API integration)
tests/client.rs -  6 tests (client library)
tests/e2e.rs    -  5 tests (end-to-end with real images)
```

Macro crate (89 tests):
```
src/lib.rs      - 49 unit tests
tests/router.rs - 21 router integration tests
tests/client.rs - 17 client integration tests
tests/expand.rs -  7 macro expansion tests
tests/ui.rs     -  5 UI/error tests
```

**Frontends compile:**
```bash
cargo check -p frontend-viewer  # OK
cargo check -p frontend-admin   # OK
```

---

## All Implemented Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/next` | Get next photo (from album or entire library) |
| GET | `/api/photos` | List all photos |
| GET | `/api/photos/{id}` | Get single photo |
| POST | `/api/photos` | Upload new photo (multipart form) |
| PUT | `/api/photos/{id}` | Update photo metadata |
| DELETE | `/api/photos/{id}` | Delete photo |
| GET | `/api/albums` | List all albums |
| GET | `/api/albums/{id}` | Get single album with photo IDs |
| POST | `/api/albums` | Create album |
| PUT | `/api/albums/{id}` | Update album |
| DELETE | `/api/albums/{id}` | Delete album |
| POST | `/api/albums/{album_id}/photos/{photo_id}` | Add photo to album |
| DELETE | `/api/albums/{album_id}/photos/{photo_id}` | Remove photo from album |
| GET | `/api/settings` | Get rotation settings |
| PUT | `/api/settings` | Update settings (select album, change interval) |
| GET | `/api/images/{id}` | Serve actual image file (binary response) |

---

## Key Files

| File | Purpose |
|------|---------|
| `src/app.rs` | App struct, API handlers, database queries |
| `src/models.rs` | SQLx database models |
| `src/test_helpers.rs` | Test utilities (in-memory DB, seeding, router with extras) |
| `src/main.rs` | Server setup, static file serving, extra routes |
| `src/on_disk_photo.rs` | Photo import/processing with ImageMagick |
| `crates/common/src/lib.rs` | Shared types and API client |
| `crates/frontend-viewer/src/main.rs` | Leptos viewer app |
| `crates/frontend-admin/src/main.rs` | Leptos admin panel |
| `migrations/*.sql` | Database schema |
| `tests/api.rs` | API integration tests (42 tests) |
| `tests/client.rs` | Client tests (6 tests) |
| `tests/e2e.rs` | End-to-end tests with real images (5 tests) |

---

## What's Left To Do

### Build & Deploy
- Build WASM frontends with trunk
- Configure static file serving for WASM bundles
- Test full end-to-end in browser

### Production Hardening
- Add body size limit configuration to production server (currently only test router has 50MB limit)
- Wire up `process_inbox()` for automatic photo import

### Optional Enhancements
- Add request body validation
- Add proper logging/tracing to handlers
