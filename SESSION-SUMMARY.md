# Session Summary: SQLx + API Macro Refactor

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

### Phase 7: Frontend Integration (Complete - This Session)
- Fixed and updated the Leptos viewer frontend
- Implemented full admin panel with Photos, Albums, and Settings tabs
- Added missing client methods for admin functionality

---

## This Session's Work

### Behavior Change: Library Fallback
When no album is selected, `/api/next` now returns photos from the entire library (ordered by ID) instead of returning an error.

### Frontend Viewer (`crates/frontend-viewer/`)
- Fixed `get_next()` client method (was calling wrong endpoint, returning wrong type)
- Updated to use server-provided interval for photo rotation
- Added metadata overlay with fade-out animation (title, artist, date, copyright)
- Uses trigger-based effect pattern for scheduling

### Frontend Admin (`crates/frontend-admin/`)
Implemented complete admin panel with three tabs:

**Photos Tab:**
- Grid display of all photos with thumbnails
- Delete functionality for each photo
- Refresh button

**Albums Tab:**
- Create new albums
- Sidebar list of albums with photo counts
- Album detail view showing:
  - Photos in album (with remove button)
  - Available photos to add (click to add)
- Delete album functionality

**Settings Tab:**
- Album selector dropdown (including "All photos" option)
- Duration presets (30s, 1m, 2m, 3m, 5m, 10m)
- Displays current settings from server

### Common Client Updates (`crates/common/`)
Added missing API client methods:
- `get_photos()` - list all photos
- `get_photo(id)` - get single photo
- `get_albums()` - list all albums
- `get_album(id)` - get single album
- `create_album(req)` - create new album
- `delete_photo(id)` - delete photo
- `delete_album(id)` - delete album
- `add_photo_to_album(album_id, photo_id)`
- `remove_photo_from_album(album_id, photo_id)`

Added `Clone` and `PartialEq` derives to types as needed for Leptos.

---

## Current State

**All 53 tests passing:**
```
tests/api.rs    - 42 tests (API integration)
tests/client.rs -  6 tests (client library)
tests/e2e.rs    -  5 tests (end-to-end with real images)
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
- Add `#[api_handler]` support for query parameters
- Add request body validation
- Add proper logging/tracing to handlers
- Photo upload in admin UI (currently API-only)
