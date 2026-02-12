# CLAUDE.md

Proc-macro library that generates axum server handlers and matching API clients from annotated impl blocks.

## Project Structure

```
api-macros/   - The proc-macro crate
api-example/  - Example implementation for testing
```

## Status

Greenfield project - initial implementation in progress.

## Problem Statement

As a backend Rust web developer, I repeatedly write the same pattern:
1. Write tokio+axum server handler functions
2. Create an API client with a function per server route
3. Server handlers call methods on `State<Arc<MyApp>>`
4. Everything uses JSON request/response bodies

This macro eliminates that boilerplate.

## Requirements

### Supported
- GET/POST/PUT/DELETE/PATCH requests
- JSON request and response bodies
- Path parameters (single or multiple)
- Query parameters (as a struct with `Serialize + Deserialize`)

### Non-goals
- Headers
- Websockets
- Non-JSON bodies

## Desired API

Annotate an impl block with `#[api]` and individual methods with `#[api_handler]`:

```rust
#[api]
impl MyServerApp {

    // #[body] marks the parameter as JSON body
    // CreateUserRequest must impl Serialize + Deserialize
    #[api_handler(method = "POST", path = "/users")]
    pub async fn create_user(&self, #[body] req: CreateUserRequest) -> MyAppResult<CreateUserResponse> {
        let user_response = todo!();
        Ok(user_response)
    }

    // #[path] marks the parameter as coming from the URL path
    #[api_handler(method = "GET", path = "/users/{id}")]
    pub async fn get_user(&self, #[path] id: UserId) -> MyAppResult<GetUserResponse> {
        let user = todo!();
        Ok(user)
    }

    // #[query] marks the parameter as query string params
    // ListUsersParams must impl Serialize + Deserialize
    #[api_handler(method = "GET", path = "/users")]
    pub async fn list_users(&self, #[query] params: ListUsersParams) -> MyAppResult<Vec<User>> {
        let users = todo!();
        Ok(users)
    }
}
```

### Type Requirements
- `UserId`: newtype over u32, impls `Display + FromStr`
- `MyAppResult<T>`: user-implemented, must impl `IntoResponse` to produce HTTP status code and JSON body

## Generated Code

### Server Handlers

```rust
async fn create_user(
    State(state): State<Arc<MyServerApp>>,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    state.create_user(req).await
}

async fn get_user(
    State(state): State<Arc<MyServerApp>>,
    Path(id): Path<UserId>,
) -> impl IntoResponse {
    state.get_user(id).await
}

async fn list_users(
    State(state): State<Arc<MyServerApp>>,
    Query(params): Query<ListUsersParams>,
) -> impl IntoResponse {
    state.list_users(params).await
}
```

### Client

```rust
pub struct MyServerAppClient {
    base_url: String,
    client: reqwest::Client,
}

impl MyServerAppClient {
    pub fn new(base_url: impl Into<String>) -> Self { ... }

    pub async fn create_user(&self, user: &CreateUserRequest) -> ClientResult<CreateUserResponse> {
        // POST to /users with JSON body
    }

    pub async fn get_user(&self, id: UserId) -> ClientResult<GetUserResponse> {
        // GET /users/{id}
    }

    pub async fn list_users(&self, params: &ListUsersParams) -> ClientResult<Vec<User>> {
        // GET /users?page=1&limit=10 (query string from params struct)
    }
}
```

Note: The client uses `serde_urlencoded` to serialize query parameters. Users of the generated client need `serde_urlencoded` as a dependency.

### Client Error Type

The client uses its own `ClientResult<T>` / `ClientError` type (not the server's `MyAppResult`) to handle:
- Network errors
- Deserialization errors
- HTTP error responses

## Commands

```bash
# Development
cargo build                    # Build all crates
cargo test                     # Run tests
cargo run -p api-example       # Run the example

# Linting & Formatting
cargo clippy                   # Lint
cargo fmt                      # Format
```

## Testing Strategy

- Unit tests in `api-macros/` for parsing logic
- `api-example/` serves as an integration test
