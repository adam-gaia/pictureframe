use api_macros::api;

pub struct MyApp;

pub type MyResult<T> = Result<T, String>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: u32,
    pub name: String,
}

#[api]
impl MyApp {
    #[api_handler(method = "POST", path = "/users")]
    pub async fn create_user(&self, #[body] req: CreateUserRequest) -> MyResult<User> {
        Ok(User {
            id: 1,
            name: req.name,
        })
    }
}

fn main() {}
