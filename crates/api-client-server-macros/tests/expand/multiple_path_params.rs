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

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PostId(pub u32);

impl std::fmt::Display for PostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for PostId {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(PostId)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Post {
    pub user_id: UserId,
    pub post_id: PostId,
    pub content: String,
}

#[api]
impl MyApp {
    #[api_handler(method = "GET", path = "/users/{user_id}/posts/{post_id}")]
    pub async fn get_user_post(
        &self,
        #[path] user_id: UserId,
        #[path] post_id: PostId,
    ) -> MyResult<Post> {
        Ok(Post {
            user_id,
            post_id,
            content: "Hello".to_string(),
        })
    }
}

fn main() {}
