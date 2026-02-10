use api_macros::api;

pub struct MyApp;

pub type MyResult<T> = Result<T, String>;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ResourceId(pub u32);

impl std::fmt::Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for ResourceId {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(ResourceId)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Resource {
    pub id: ResourceId,
    pub value: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CreateRequest {
    pub value: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UpdateRequest {
    pub value: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PatchRequest {
    pub value: Option<String>,
}

#[api]
impl MyApp {
    #[api_handler(method = "GET", path = "/resources")]
    pub async fn list_resources(&self) -> MyResult<Vec<Resource>> {
        Ok(vec![])
    }

    #[api_handler(method = "POST", path = "/resources")]
    pub async fn create_resource(&self, #[body] req: CreateRequest) -> MyResult<Resource> {
        Ok(Resource {
            id: ResourceId(1),
            value: req.value,
        })
    }

    #[api_handler(method = "GET", path = "/resources/{id}")]
    pub async fn get_resource(&self, #[path] id: ResourceId) -> MyResult<Resource> {
        Ok(Resource {
            id,
            value: "test".to_string(),
        })
    }

    #[api_handler(method = "PUT", path = "/resources/{id}")]
    pub async fn update_resource(
        &self,
        #[path] id: ResourceId,
        #[body] req: UpdateRequest,
    ) -> MyResult<Resource> {
        Ok(Resource { id, value: req.value })
    }

    #[api_handler(method = "PATCH", path = "/resources/{id}")]
    pub async fn patch_resource(
        &self,
        #[path] id: ResourceId,
        #[body] req: PatchRequest,
    ) -> MyResult<Resource> {
        Ok(Resource {
            id,
            value: req.value.unwrap_or_default(),
        })
    }

    #[api_handler(method = "DELETE", path = "/resources/{id}")]
    pub async fn delete_resource(&self, #[path] id: ResourceId) -> MyResult<()> {
        Ok(())
    }
}

fn main() {}
