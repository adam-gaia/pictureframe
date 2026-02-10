use api_macros::api;

pub struct MyApp;

#[api]
impl MyApp {
    #[api_handler(method = "POST", path = "/users")]
    pub async fn create_user(&self, req: String) -> Result<String, String> {
        //                         ^^^ missing #[body] or #[path] annotation
        Ok(req)
    }
}

fn main() {}
