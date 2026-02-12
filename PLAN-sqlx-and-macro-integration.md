# Combined Refactor Plan: SQLx Migration + API Macro Integration

## Overview

The macro integration is partially started but most handlers are stubs/commented out. Since we need to implement the actual business logic anyway, it makes sense to **migrate to SQLx first**, then implement the handlers using SQLx queries.

---

## Phase 1: SQLx Infrastructure Setup

### Step 1.1: Update Cargo.toml dependencies
- Remove: `sea-orm`, `entity` (path dep), `migration` (path dep)
- Add: `sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "macros", "chrono"] }`

### Step 1.2: Create SQLx migrations directory
- Create `migrations/` directory in project root
- Create `migrations/001_create_photos.sql` (from SeaORM migration)
- Create `migrations/002_create_albums.sql`
- Create `migrations/003_create_album_photos.sql` (junction table for many-to-many)
- Create `migrations/004_create_settings.sql` (for persisting rotation settings)

### Step 1.3: Create model structs
- Create `src/models.rs` with plain structs + `sqlx::FromRow` derives
- Define: `DbPhoto`, `DbAlbum`, `DbAlbumPhoto`, `DbSettings`

### Step 1.4: Update App struct
- Replace `sea_orm::DatabaseConnection` with `sqlx::SqlitePool`
- Update `App::new()` to use `SqlitePool::connect()` and `sqlx::migrate!()`
- Remove SeaORM migration call

### Step 1.5: Delete SeaORM crates
- Remove `crates/entity/` directory
- Remove `crates/migration/` directory
- Update workspace members in root `Cargo.toml`

---

## Phase 2: Database Query Layer

### Step 2.1: Implement photo queries
- `get_photo_by_id(id: i32) -> Option<DbPhoto>`
- `get_photos() -> Vec<DbPhoto>`
- `insert_photo(photo: NewPhoto) -> DbPhoto`
- `update_photo(id: i32, updates: UpdatePhotoRequest)`
- `delete_photo(id: i32)`

### Step 2.2: Implement album queries
- `get_album_by_id(id: i32) -> Option<DbAlbum>`
- `get_albums() -> Vec<DbAlbum>`
- `get_album_photos(album_id: i32) -> Vec<DbPhoto>`
- `insert_album(album: NewAlbum) -> DbAlbum`
- `add_photo_to_album(album_id: i32, photo_id: i32)`
- `remove_photo_from_album(album_id: i32, photo_id: i32)`

### Step 2.3: Implement settings queries
- `get_settings() -> DbSettings`
- `update_settings(updates: UpdateSettingsRequest)`

---

## Phase 3: API Handler Implementation

### Step 3.1: Ensure APIResult type is complete
- `src/app.rs` already has `APIResult<T>` enum
- Verify it works with the macro (returns proper HTTP responses)

### Step 3.2: Implement photo handlers
```rust
#[api_handler(method = "GET", path = "/api/photos")]
pub async fn get_photos(&self) -> APIResult<Vec<Photo>>

#[api_handler(method = "GET", path = "/api/photos/next")]
pub async fn get_next_photo(&self) -> APIResult<Next>

#[api_handler(method = "GET", path = "/api/photos/{id}")]
pub async fn get_photo(&self, #[path] id: i32) -> APIResult<Photo>

#[api_handler(method = "PUT", path = "/api/photos/{id}")]
pub async fn update_photo(&self, #[path] id: i32, #[body] updates: UpdatePhotoRequest) -> APIResult<()>

#[api_handler(method = "DELETE", path = "/api/photos/{id}")]
pub async fn delete_photo(&self, #[path] id: i32) -> APIResult<()>
```

### Step 3.3: Implement album handlers
```rust
#[api_handler(method = "GET", path = "/api/albums")]
pub async fn get_albums(&self) -> APIResult<Vec<Album>>

#[api_handler(method = "GET", path = "/api/albums/{id}")]
pub async fn get_album(&self, #[path] id: i32) -> APIResult<Album>

#[api_handler(method = "POST", path = "/api/albums")]
pub async fn create_album(&self, #[body] req: CreateAlbumRequest) -> APIResult<Album>
```

### Step 3.4: Implement settings handlers
```rust
#[api_handler(method = "GET", path = "/api/settings")]
pub async fn get_settings(&self) -> APIResult<RotationSettings>

#[api_handler(method = "PUT", path = "/api/settings")]
pub async fn update_settings(&self, #[body] updates: UpdateSettingsRequest) -> APIResult<()>
```

### Step 3.5: Implement image serving
```rust
#[api_handler(method = "GET", path = "/api/images/{id}")]
pub async fn get_image(&self, #[path] id: i32) -> impl IntoResponse
// Note: May need special handling for binary responses - check if macro supports this
```

---

## Phase 4: Clean Up Old Code

### Step 4.1: Remove old api module
- Delete `src/api/` directory if it exists
- Delete `src/api.rs` module declaration

### Step 4.2: Update main.rs
- Remove old manual routing (already done)
- Ensure `state.router()` is used (already done)
- Add static file serving back (currently commented out)

### Step 4.3: Update imports
- Remove `entity::` and `migration::` imports
- Update `sea_orm::` imports to `sqlx::`

---

## Phase 5: Testing & Verification

### Step 5.1: Verify macro expansion
```bash
cargo expand -p pictureframe > expanded.rs
```

### Step 5.2: Test compilation
```bash
cargo build
```

### Step 5.3: Run the server and test endpoints
```bash
cargo run
curl http://localhost:3000/_health
curl http://localhost:3000/api/photos
```

### Step 5.4: Test with generated client (optional)
- Write integration tests using `AppClient` generated by macro

---

## File Changes Summary

| File | Action |
|------|--------|
| `Cargo.toml` | Remove sea-orm, entity, migration deps; add sqlx |
| `migrations/*.sql` | Create (new SQLx migration files) |
| `src/models.rs` | Create (new database models) |
| `src/app.rs` | Refactor: SQLx pool, implement handlers |
| `src/main.rs` | Minor updates (remove old imports) |
| `crates/entity/` | Delete |
| `crates/migration/` | Delete |

---

## Recommended Order of Execution

1. **Phase 1** - Get SQLx compiling with empty queries
2. **Phase 2** - Implement one vertical slice (photos: model + queries + handler)
3. **Phase 3** - Iterate: add albums, settings
4. **Phase 4** - Clean up
5. **Phase 5** - Test everything
