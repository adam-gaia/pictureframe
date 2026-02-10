use api_macros::api;

pub struct MyApp;

pub type MyResult<T> = Result<T, String>;

#[api]
impl MyApp {
    #[api_handler(method = "GET", path = "/health")]
    pub async fn health(&self) -> MyResult<String> {
        Ok("ok".to_string())
    }
}

fn main() {}
