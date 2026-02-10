use api_macros::api;

pub struct MyApp;

pub type MyResult<T> = Result<T, String>;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: String,
}

#[api]
impl MyApp {
    #[api_handler(method = "PUT", path = "/users/{id}")]
    pub async fn update_user(
        &self,
        #[path] id: UserId,
        #[body] req: UpdateUserRequest,
    ) -> MyResult<User> {
        Ok(User {
            id,
            name: req.name,
            email: req.email,
        })
    }
}

fn main() {}
