use std::{net::SocketAddr, str::FromStr};

use carbon_http_server::HttpServer;

#[tokio::main]
async fn main() {
    let server = HttpServer::new(SocketAddr::from_str("127.0.0.1:8080").unwrap());
    server.serve().await.unwrap();
}
