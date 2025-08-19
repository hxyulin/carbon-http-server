//! An async HTTP server implementation in rust

#![feature(async_fn_traits, slice_split_once, str_from_raw_parts)]

pub mod http;
pub mod service;
pub mod sync;

use std::{net::SocketAddr, num::NonZeroUsize, sync::Arc, time::Duration};

use crate::http::{
    HttpVersion,
    header::{Connection, ConnectionType},
    parser::{HttpParseError, Parser, Sender},
    request::Request,
    response::{Response, ResponseBuilder, StatusCode},
};
use tokio::net::{TcpSocket, TcpStream};

#[derive(Debug, Clone)]
pub struct HttpServerConfig {
    // Request head (request-line + headers)
    pub max_request_line_bytes: NonZeroUsize, // e.g., "GET /... HTTP/1.1" + CRLF
    pub max_header_bytes_total: NonZeroUsize, // all header lines + CRLFs (not body)
    pub max_header_line_bytes: NonZeroUsize,  // any single header line
    pub max_header_count: NonZeroUsize,       // total header fields

    // Origin-form specifics (optional but handy)
    pub max_path_bytes: NonZeroUsize,
    pub max_query_bytes: NonZeroUsize,

    // Body (message payload)
    pub max_body_bytes: Option<NonZeroUsize>, // None = unlimited (let app decide)
    pub max_chunk_size_bytes: NonZeroUsize,   // for chunked encoding
    pub max_trailer_bytes_total: NonZeroUsize, // trailers after chunked body

    // Timeouts (doS/smurf protection)
    pub header_read_timeout: Duration,
    pub request_body_timeout: Duration,
    pub keep_alive_timeout: Duration,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            // head
            max_request_line_bytes: NonZeroUsize::new(8 * 1024).unwrap(), // 8 KiB
            max_header_bytes_total: NonZeroUsize::new(64 * 1024).unwrap(), // 64 KiB
            max_header_line_bytes: NonZeroUsize::new(8 * 1024).unwrap(),  // 8 KiB
            max_header_count: NonZeroUsize::new(100).unwrap(),

            // target subparts
            max_path_bytes: NonZeroUsize::new(4 * 1024).unwrap(), // 4 KiB
            max_query_bytes: NonZeroUsize::new(8 * 1024).unwrap(), // 8 KiB

            // body
            max_body_bytes: None,
            max_chunk_size_bytes: NonZeroUsize::new(8 * 1024 * 1024).unwrap(), // 8 MiB
            max_trailer_bytes_total: NonZeroUsize::new(8 * 1024).unwrap(),     // 8 KiB

            // timeouts
            header_read_timeout: Duration::from_secs(10),
            request_body_timeout: Duration::from_secs(60),
            keep_alive_timeout: Duration::from_secs(75),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HttpServerError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    HttpParseError(#[from] HttpParseError),
}

pub type HttpServerResult<T> = Result<T, HttpServerError>;

pub struct HttpServer<R: Router>(Arc<HttpServerInternal<R>>);

impl<R: Router> HttpServer<R> {
    pub fn new<A: Into<SocketAddr>>(addr: A, router: R) -> Self {
        Self(Arc::new(HttpServerInternal::new(addr, router)))
    }

    pub async fn serve(&self) -> Result<(), HttpServerError> {
        HttpServerInternal::serve(self.0.clone()).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RouterError {
    #[error(transparent)]
    Generic(#[from] Box<dyn std::error::Error + Send + Sync>),
}

pub trait Router: Send + Sync + 'static {
    fn route(
        &self,
        request: &Request,
    ) -> impl Future<Output = Result<Response, RouterError>> + Send;
}

pub(crate) struct HttpServerInternal<R: Router> {
    addr: SocketAddr,
    router: R,
}

impl<R: Router> HttpServerInternal<R> {
    pub fn new<A: Into<SocketAddr>>(addr: A, router: R) -> Self {
        Self {
            addr: addr.into(),
            router,
        }
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

    async fn handle_connection(sel: Arc<Self>, stream: TcpStream, addr: SocketAddr) {
        if let Err(err) = sel.handle_connection_internal(stream, addr).await {
            log::error!("server error: {}", err);
        }
    }

    async fn handle_connection_internal(
        &self,
        mut stream: TcpStream,
        addr: SocketAddr,
    ) -> HttpServerResult<()> {
        let (mut read_stream, mut write_stream) = stream.split();
        let mut parser = Parser::new(&mut read_stream);
        let mut sender = Sender::new(&mut write_stream);

        loop {
            let req = match parser.parse_request().await {
                Ok(mut req) => {
                    req.remote = Some(addr);
                    req
                }
                Err(err) => {
                    log::error!("failed to parse request: {}", err);
                    let res = ResponseBuilder::new(HttpVersion::HTTP_1_1, err.status_code())
                        .set_header::<Connection>(ConnectionType::Close)
                        .build();
                    sender.send_response(res).await?;
                    return Ok(());
                }
            };
            let close_connection = matches!(
                req.headers.get_header::<Connection>().unwrap(),
                Some(ConnectionType::Close)
            );
            let res = self.router.route(&req).await;
            match res {
                Ok(res) => {
                    let close_connection = matches!(
                        res.headers.get_header::<Connection>().unwrap(),
                        Some(ConnectionType::Close)
                    );
                    log::debug!("sending response = {:#?}", res);
                    sender.send_response(res).await?;
                    if close_connection {
                        return Ok(());
                    }
                }
                Err(err) => {
                    let res = ResponseBuilder::from_req(&req, StatusCode::INTERNAL_SERVER_ERROR)
                        .set_header::<Connection>(ConnectionType::Close)
                        .build();
                    sender.send_response(res).await?;
                    log::error!("router error: {}", err)
                }
            }

            if close_connection {
                return Ok(());
            }
        }
    }
}

pub fn init_logger() {
    env_logger::init();
}
