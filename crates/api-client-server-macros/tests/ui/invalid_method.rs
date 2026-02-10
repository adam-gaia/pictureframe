use api_macros::api;

pub struct MyApp;

#[api]
impl MyApp {
    #[api_handler(method = "TRACE", path = "/debug")]
    pub async fn trace_endpoint(&self) -> Result<String, String> {
        // TRACE is not a supported HTTP method
        Ok("trace".to_string())
    }
}

fn main() {}
