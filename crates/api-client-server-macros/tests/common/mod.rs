//! Shared types and server application for integration tests.

use api_macros::api;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

// ── Domain types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UserId(pub u32);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for UserId {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(UserId)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub id: UserId,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetUserResponse {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ListUsersParams {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListUsersResponse {
    pub users: Vec<GetUserResponse>,
    pub page: u32,
    pub limit: u32,
}

// ── Result type (user-defined, must impl IntoResponse) ─────────────────

pub enum MyAppResult<T> {
    Ok(T),
    NotFound(String),
    InternalError(String),
}

// Convenience: allow `?` usage in handlers
impl<T> From<T> for MyAppResult<T> {
    fn from(val: T) -> Self {
        MyAppResult::Ok(val)
    }
}

impl<T: Serialize> IntoResponse for MyAppResult<T> {
    fn into_response(self) -> axum::response::Response {
        match self {
            MyAppResult::Ok(val) => (StatusCode::OK, axum::Json(val)).into_response(),
            MyAppResult::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "error": msg })),
            )
                .into_response(),
            MyAppResult::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({ "error": msg })),
            )
                .into_response(),
        }
    }
}

// ── The server application ─────────────────────────────────────────────

pub struct MyServerApp {
    // In real code: database pool, config, etc.
}

#[api]
impl MyServerApp {
    #[api_handler(method = "POST", path = "/users")]
    pub async fn create_user(
        &self,
        #[body] req: CreateUserRequest,
    ) -> MyAppResult<CreateUserResponse> {
        // Your real logic here
        let response = CreateUserResponse {
            id: UserId(42),
            name: req.name,
        };
        MyAppResult::Ok(response)
    }

    #[api_handler(method = "GET", path = "/users/{id}")]
    pub async fn get_user(&self, #[path] id: UserId) -> MyAppResult<GetUserResponse> {
        // Your real logic here
        MyAppResult::Ok(GetUserResponse {
            id,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        })
    }

    #[api_handler(method = "DELETE", path = "/users/{id}")]
    #[allow(unused_variables)]
    pub async fn delete_user(&self, #[path] id: UserId) -> MyAppResult<()> {
        // Your real logic here
        MyAppResult::Ok(())
    }

    #[api_handler(method = "GET", path = "/users")]
    pub async fn list_users(&self, #[query] params: ListUsersParams) -> MyAppResult<ListUsersResponse> {
        // Your real logic here
        MyAppResult::Ok(ListUsersResponse {
            users: vec![
                GetUserResponse {
                    id: UserId(1),
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                },
                GetUserResponse {
                    id: UserId(2),
                    name: "Bob".to_string(),
                    email: "bob@example.com".to_string(),
                },
            ],
            page: params.page.unwrap_or(1),
            limit: params.limit.unwrap_or(10),
        })
    }

    // Non-annotated methods are left untouched
    #[allow(dead_code)]
    pub fn some_internal_helper(&self) -> String {
        "I'm not an API handler".to_string()
    }
}
