use api_macros::api;

pub struct MyApp;

#[api]
impl MyApp {
    #[api_handler(method = "GET", path = "/health")]
    pub async fn health(&self) {
        // Missing return type
    }
}

fn main() {}
