# Plan: Integrate api-client-server-macros

This document outlines the steps to refactor the frame2 server and client to use the `api-client-server-macros` procedural macro crate.

## Current State

### Server Side
- **Handler functions**: Defined as free functions in `src/api/{albums,photos,settings}.rs`
- **Business logic**: Lives in `src/app.rs` on the `App` struct
- **Routing**: Manually wired in `src/main.rs` using nested routers
- **Pattern**: Handlers call methods on `State<Arc<App>>`

### Client Side
- **Location**: `crates/common/src/lib.rs`
- **HTTP library**: `reqwasm` (WASM-compatible for frontend)
- **Structure**: Manual `Client` struct with `get/post/put/delete` helpers

### Macro Capabilities
The `#[api]` macro generates:
1. Axum handler functions from `#[api_handler(...)]` annotated methods
2. A `.router()` method that wires all routes
3. A `{StructName}Client` with matching methods (uses `reqwest`)

---

## Key Considerations

### WASM Client Compatibility
The macro generates a `reqwest`-based client, but the current client uses `reqwasm` for WASM frontend compatibility. Options:

1. **Keep both clients**: Use macro-generated client for server-side/CLI usage, keep manual reqwasm client for frontends
2. **Feature-flag the client**: Modify the macro to conditionally generate reqwest vs reqwasm
3. **Use reqwest with WASM**: reqwest supports WASM via feature flags, but may have different API

**Recommendation**: Keep the existing reqwasm client in `common` for now. The macro-generated client can be used for integration tests and any server-side API calls.

### Result Type
The macro expects handlers to return a type like `MyAppResult<T>` that implements `IntoResponse`. Current handlers use `Result<T>` from anyhow and manually unwrap.

**Required**: Create an `AppResult<T>` type that:
- Wraps `Result<T, AppError>`
- Implements `IntoResponse` (returns appropriate HTTP status codes)
- Has `AppError` that covers all error cases

---

## Implementation Steps

### Phase 1: Add Dependencies and Error Types

#### Step 1.1: Add macro crate dependency
Add to `Cargo.toml`:
```toml
[dependencies]
api-client-server-macros = { path = "crates/api-client-server-macros" }
```

#### Step 1.2: Create AppResult/AppError types
Create `src/error.rs`:
```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Internal(anyhow::Error),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

// Make AppResult<T> work as IntoResponse when T: Serialize
impl<T: Serialize> IntoResponse for AppResult<T> {
    // ... implementation
}
```

**Note**: Axum's `IntoResponse` is already implemented for `Result<T, E>` where both implement `IntoResponse`. The above may need adjustment.

---

### Phase 2: Annotate App Methods

#### Step 2.1: Add `#[api]` to the App impl block
In `src/app.rs`, annotate the impl block:
```rust
use api_client_server_macros::{api, api_handler};

#[api]
impl App {
    // ... existing methods
}
```

#### Step 2.2: Annotate API methods with `#[api_handler]`
Convert each API endpoint. Example transformations:

**Before** (`src/api/photos.rs`):
```rust
pub async fn get_photos(State(state): State<Arc<App>>) -> Json<Vec<Photo>> {
    let db_photos = state.get_photos().await.unwrap();
    let photos: Vec<Photo> = db_photos.into_iter().map(db_photo_to_photo).collect();
    Json(photos)
}
```

**After** (`src/app.rs`):
```rust
#[api_handler(method = "GET", path = "/api/photos")]
pub async fn get_photos(&self) -> AppResult<Vec<Photo>> {
    let db_photos = self.get_photos_internal().await?;
    let photos: Vec<Photo> = db_photos.into_iter().map(db_photo_to_photo).collect();
    Ok(photos)
}
```

#### Step 2.3: Complete method annotations

| Current Handler | Method | Path | Parameters |
|-----------------|--------|------|------------|
| `get_next_photo` | GET | `/api/photos/next` | none |
| `get_photos` | GET | `/api/photos` | none |
| `update_photo` | PUT | `/api/photos/{id}` | `#[path] id`, `#[body] updates` |
| `upload_photo` | POST | `/api/photos` | `#[body] photo_req` |
| `get_albums` | GET | `/api/albums` | none |
| `get_settings` | GET | `/api/settings` | none |
| `update_settings` | PUT | `/api/settings` | `#[body] updates` |

---

### Phase 3: Refactor App Structure

#### Step 3.1: Move conversion functions to App
Functions like `db_photo_to_photo` and `db_album_to_album` should move into the App impl or become associated functions.

#### Step 3.2: Rename internal methods
To avoid name collisions, rename internal methods:
- `get_photos()` → `get_photos_from_db()` (internal)
- The `#[api_handler]` method becomes the public `get_photos()`

#### Step 3.3: Handle the response wrapping
Current handlers manually wrap responses in `Json<T>`. The macro expects the method to return the type directly (wrapped in `AppResult<T>`), and the generated handler adds the `Json` wrapper.

---

### Phase 4: Update Routing

#### Step 4.1: Replace manual routing in main.rs

**Before**:
```rust
let api_photos_router = Router::new()
    .route("/next", get(get_next_photo))
    .route("/", get(get_photos));
let api_albums_router = Router::new().route("/", get(get_albums));
let api_router = Router::new()
    .nest("/photos", api_photos_router)
    .nest("/albums", api_albums_router);

let app = Router::new()
    .route("/_health", get(health))
    .nest("/api", api_router)
    // ...
    .with_state(state);
```

**After**:
```rust
let api_router = state.clone().router();  // Generated by macro

let app = Router::new()
    .route("/_health", get(health))
    .merge(api_router)  // Contains all /api/* routes
    // ...
```

#### Step 4.2: Handle non-API routes
The `/_health` endpoint and static file serving remain outside the macro. Either:
- Keep them separate and merge routers
- Or add health check to the `#[api]` impl block

---

### Phase 5: Delete Old Code

#### Step 5.1: Remove old handler modules
Delete:
- `src/api/albums.rs`
- `src/api/photos.rs`
- `src/api/settings.rs`
- `src/api.rs` (the module declaration file)

#### Step 5.2: Clean up imports in main.rs
Remove imports of the old handler functions.

---

### Phase 6: Update Client (Optional)

The macro generates `AppClient` and `AppClientError`. Decide whether to:

#### Option A: Keep existing reqwasm client
- Frontends continue using `common::Client`
- Macro-generated client used for tests/CLI tools

#### Option B: Migrate to generated client
- Requires making reqwest work in WASM context
- Add to frontend Cargo.toml: `reqwest = { version = "...", features = ["wasm"] }`
- Replace `common::Client` usage with `AppClient`

#### Option C: Dual client generation (future macro enhancement)
- Modify the macro to support both reqwest and reqwasm
- Out of scope for initial integration

---

### Phase 7: Testing

#### Step 7.1: Verify macro expansion
```bash
cargo expand -p frame2 > expanded.rs
```
Review the generated handlers, router, and client.

#### Step 7.2: Integration tests
Use the generated `AppClient` to write integration tests:
```rust
#[tokio::test]
async fn test_get_photos() {
    let app = spawn_test_server().await;
    let client = AppClient::new(&app.url);

    let photos = client.get_photos().await.unwrap();
    assert!(photos.is_empty());
}
```

#### Step 7.3: Manual testing
```bash
cargo run
curl http://localhost:3000/api/photos
```

---

## File Changes Summary

| File | Action |
|------|--------|
| `Cargo.toml` | Add api-client-server-macros dependency |
| `src/error.rs` | Create (new file with AppError/AppResult) |
| `src/app.rs` | Add `#[api]` and `#[api_handler]` annotations |
| `src/main.rs` | Replace manual routing with `.router()` |
| `src/api.rs` | Delete |
| `src/api/albums.rs` | Delete |
| `src/api/photos.rs` | Delete |
| `src/api/settings.rs` | Delete |

---

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Macro compilation errors are hard to debug | Use `cargo expand` to see generated code |
| Breaking WASM frontend | Keep existing reqwasm client, test frontend separately |
| Missing features (query params, headers) | Accept limitation or extend macro later |
| Path conflicts with nested routers | Ensure all paths in `#[api_handler]` are fully qualified (e.g., `/api/photos` not `/photos`) |

---

## Future Enhancements

1. **Query parameter support**: Add `#[query]` annotation
2. **WASM client generation**: Feature-flagged reqwasm support
3. **OpenAPI generation**: Generate OpenAPI spec from annotations
4. **Middleware support**: Per-route middleware annotations
