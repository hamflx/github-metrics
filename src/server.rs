use poem::{endpoint::StaticFilesEndpoint, listener::TcpListener, Route, Server};
use poem_openapi::{
    payload::Json,
    types::{ParseFromJSON, ToJSON},
    Object, OpenApi, OpenApiService,
};
use serde::{Deserialize, Serialize};

use crate::{
    github::{read_persisted_stats, GitHubRepoStats},
    version::VERSION,
};

pub struct WebServer {
    api: Api,
}

impl WebServer {
    pub fn new(db_file: &str) -> Self {
        Self {
            api: Api(db_file.to_owned()),
        }
    }

    pub async fn run(self, port: u16) -> Result<(), std::io::Error> {
        let api_service = OpenApiService::new(self.api, "GitHub Metrics Sync", VERSION)
            .server(format!("http://localhost:{}/api", port));
        let ui = api_service.swagger_ui();

        Server::new(TcpListener::bind(format!("127.0.0.1:{}", port)))
            .run(
                Route::new()
                    .nest("/api", api_service)
                    .nest("/swagger", ui)
                    .nest(
                        "/",
                        StaticFilesEndpoint::new("web").index_file("index.html"),
                    ),
            )
            .await
    }
}

struct Api(String);

#[OpenApi]
impl Api {
    #[oai(path = "/traffics", method = "get")]
    async fn index(&self) -> Json<ApiResult<GitHubRepoStats>> {
        match read_persisted_stats(self.0.as_str()) {
            Ok(stats) => Json(ApiResult::ok(stats)),
            Err(e) => Json(ApiResult::err(e.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(inline)]
struct ApiResult<T: ToJSON + ParseFromJSON> {
    pub code: Option<String>,
    pub message: Option<String>,
    pub data: Option<T>,
}

impl<T: ToJSON + ParseFromJSON> ApiResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            code: Some("ok".to_string()),
            message: None,
            data: Some(data),
        }
    }

    pub fn err(message: String) -> Self {
        Self {
            code: Some("err".to_string()),
            message: Some(message),
            data: None,
        }
    }
}
