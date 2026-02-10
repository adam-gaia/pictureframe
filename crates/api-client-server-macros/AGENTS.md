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
- GET/POST/PUT/DELETE requests
- JSON request and response bodies
- Single path parameter per route

### Non-goals
- Headers
- Query parameters
- Websockets
- Multiple path parameters
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
```

### Client

```rust
pub struct MyServerAppClient {
    base_url: String,
    client: reqwest::Client,
}

impl MyServerAppClient {
    pub fn new(base_url: impl Into<String>) -> Self { ... }

    // Private helpers: get, post, put, delete

    pub async fn create_user(&self, user: CreateUserRequest) -> ClientResult<CreateUserResponse> {
        self.post("/users", &user).await
    }

    pub async fn get_user(&self, id: UserId) -> ClientResult<GetUserResponse> {
        self.get(&format!("/users/{id}")).await
    }
}
```

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
