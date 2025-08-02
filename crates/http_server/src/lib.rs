//! An async HTTP server implementation in rust

#![feature(async_fn_traits, slice_split_once)]

pub mod http;
pub mod sync;

use std::{net::SocketAddr, sync::Arc};

use crate::http::request::{HttpRequestParser, RequestParseError};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpSocket, TcpStream},
};

#[derive(Debug, thiserror::Error)]
pub enum HttpServerError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    RequestParseError(#[from] RequestParseError),
}

pub type HttpServerResult<T> = Result<T, HttpServerError>;

pub struct HttpServer(Arc<HttpServerInternal>);

impl HttpServer {
    pub fn new<A: Into<SocketAddr>>(addr: A) -> Self {
        Self(Arc::new(HttpServerInternal::new(addr)))
    }

    pub async fn serve(&self) -> Result<(), HttpServerError> {
        HttpServerInternal::serve(self.0.clone()).await
    }
}

pub(crate) struct HttpServerInternal {
    addr: SocketAddr,
}

impl HttpServerInternal {
    pub fn new<A: Into<SocketAddr>>(addr: A) -> Self {
        Self { addr: addr.into() }
    }

    pub async fn serve(sel: Arc<Self>) -> Result<(), HttpServerError> {
        let sock = match sel.addr {
            SocketAddr::V4(_) => TcpSocket::new_v4()?,
            SocketAddr::V6(_) => TcpSocket::new_v6()?,
        };

        sock.set_reuseaddr(true)?;
        sock.bind(sel.addr)?;

        let listener = sock.listen(1024)?;
        loop {
            let (stream, addr) = listener.accept().await?;
            tokio::spawn(HttpServerInternal::handle_connection(
                sel.clone(),
                stream,
                addr,
            ));
        }
    }

    async fn handle_connection(sel: Arc<Self>, mut stream: TcpStream, addr: SocketAddr) {
        if let Err(err) = sel.handle_connection_internal(&mut stream, addr).await {
            eprintln!("error: {err}");

            let err = stream
                .write_all(b"HTTP/1.1 500 Internal Server Error")
                .await;
            if let Err(err) = err {
                eprintln!("failed to write error to server: {err}");
            }
        }
    }

    async fn handle_connection_internal(
        &self,
        stream: &mut TcpStream,
        addr: SocketAddr,
    ) -> HttpServerResult<()> {
        let req = HttpRequestParser::parse(&mut *stream).await?;
        println!("req = {:#?}", req);

        let response = b"\
HTTP/1.1 200 OK\r\n\
Content-Type: text/plain\r\n\
Content-Length: 13\r\n\
Connection: close\r\n\
\r\n\
Hello, world!";
        stream.write_all(response).await?;

        stream.flush().await?;
        Ok(())
    }
}
