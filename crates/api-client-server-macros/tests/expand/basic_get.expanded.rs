use api_macros::api;
pub struct MyApp;
pub type MyResult<T> = Result<T, String>;
impl MyApp {
    pub async fn health(&self) -> MyResult<String> {
        Ok("ok".to_string())
    }
}
impl MyApp {
    /// Build an axum Router with all annotated handlers wired up.
    pub fn router(self: std::sync::Arc<Self>) -> axum::Router {
        axum::Router::new()
            .route("/health", axum::routing::get(__axum_handler_health))
            .with_state(self)
    }
}
async fn __axum_handler_health(
    axum::extract::State(state): axum::extract::State<std::sync::Arc<MyApp>>,
) -> impl axum::response::IntoResponse {
    state.health().await
}
/// Auto-generated HTTP client for the API.
pub struct MyAppClient {
    base_url: String,
    client: reqwest::Client,
}
pub enum MyAppClientError {
    Request(reqwest::Error),
    Api { status: reqwest::StatusCode, body: String },
}
#[automatically_derived]
impl ::core::fmt::Debug for MyAppClientError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            MyAppClientError::Request(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(
                    f,
                    "Request",
                    &__self_0,
                )
            }
            MyAppClientError::Api { status: __self_0, body: __self_1 } => {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "Api",
                    "status",
                    __self_0,
                    "body",
                    &__self_1,
                )
            }
        }
    }
}
impl std::fmt::Display for MyAppClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(e) => f.write_fmt(format_args!("HTTP request error: {0}", e)),
            Self::Api { status, body } => {
                f.write_fmt(format_args!("API error ({0}): {1}", status, body))
            }
        }
    }
}
impl std::error::Error for MyAppClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(e) => Some(e),
            Self::Api { .. } => None,
        }
    }
}
impl MyAppClient {
    /// Create a new client pointing at the given base URL (e.g. `"http://localhost:3000"`).
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }
    /// Create a new client with a custom `reqwest::Client`.
    pub fn with_client(base_url: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            base_url: base_url.into(),
            client,
        }
    }
    pub async fn health(&self) -> Result<String, MyAppClientError> {
        let url = ::alloc::__export::must_use({
            ::alloc::fmt::format(format_args!("{0}{1}", self.base_url, "/health"))
        });
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(MyAppClientError::Request)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(MyAppClientError::Api {
                status,
                body,
            });
        }
        response.json::<String>().await.map_err(MyAppClientError::Request)
    }
}
fn main() {}
