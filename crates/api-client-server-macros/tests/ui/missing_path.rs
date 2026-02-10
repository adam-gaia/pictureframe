use api_macros::api;

pub struct MyApp;

#[api]
impl MyApp {
    #[api_handler(method = "GET")]
    pub async fn health(&self) -> Result<String, String> {
        // Missing path attribute
        Ok("ok".to_string())
    }
}

fn main() {}
