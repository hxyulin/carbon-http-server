use std::{
    env::current_dir,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
};

use carbon_http_server::{HttpServer, Router};
use tokio::io::AsyncWriteExt;

pub struct FileServer {
    root: PathBuf,
}

impl Router for FileServer {
    async fn route(
        &self,
        stream: &mut tokio::net::TcpStream,
        request: carbon_http_server::http::request::Request,
    ) {
    }
}

#[tokio::main]
async fn main() {
    let fs = FileServer {
        root: current_dir().expect("failed to get cwd"),
    };
    let server = HttpServer::new(SocketAddr::from_str("127.0.0.1:8080").unwrap(), fs);
    server.serve().await.unwrap();
}
