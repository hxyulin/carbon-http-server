use std::{
    env::current_dir, net::SocketAddr, path::PathBuf, str::FromStr
};

use bytes::Bytes;
use carbon_http_server::{http::{request::Request, response::{Response, ResponseBuilder, StatusCode}}, init_logger, HttpServer, Router, RouterError};

pub struct FileServer {
    root: PathBuf,
}

impl Router for FileServer {
    async fn route(
        &self,
        request: &Request,
    ) -> Result<Response, RouterError> {
        log::debug!("request = {:#?}", request);
        let target = request.target().unwrap();
        let path = self.root.join(target.as_str().strip_prefix("/").unwrap());
        if !path.is_file() {
            return Ok(ResponseBuilder::from_req(request, StatusCode::NOT_FOUND)
                .body(Bytes::from_static(b"file not found"))
                .build());
        }
        let data = std::fs::read(path).unwrap();
        Ok(
            ResponseBuilder::from_req(request, StatusCode::OK)
                .body(Bytes::from(data))
                .build()
        )
    }
}

#[tokio::main]
async fn main() {
    init_logger();
    let fs = FileServer {
        root: current_dir().expect("failed to get cwd"),
    };
    let server = HttpServer::new(SocketAddr::from_str("127.0.0.1:8080").unwrap(), fs);
    server.serve().await.unwrap();
}
