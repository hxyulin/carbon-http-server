//! An async HTTP server implementation in rust

use std::{net::SocketAddr, str::FromStr, sync::Arc};

use ascii::AsciiString;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    net::{TcpSocket, TcpStream},
};

#[derive(Debug, thiserror::Error)]
pub enum HttpServerError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

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

    async fn handle_connection(
        sel: Arc<Self>,
        mut stream: TcpStream,
        addr: SocketAddr,
    ) -> std::io::Result<()> {
        let req = HttpRequest::parse(&mut stream).await?;
        println!("Req: {:#?}", req);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    CONNECT,
    TRACE,
    Extension(AsciiString),
}

impl<'a> From<&'a str> for Method {
    fn from(value: &'a str) -> Self {
        match value {
            "GET" => Self::GET,
            "POST" => Self::POST,
            "PUT" => Self::PUT,
            "DELETE" => Self::DELETE,
            "PATCH" => Self::PATCH,
            "OPTIONS" => Self::OPTIONS,
            "CONNECT" => Self::CONNECT,
            "TRACE" => Self::TRACE,
            other => Self::Extension(AsciiString::from_str(other).unwrap()),
        }
    }
}

enum RequestParseState {
    StatusLine,
    Headers,
    Body,
    Done,
}

impl RequestParseState {
    fn next(self) -> Self {
        match self {
            Self::StatusLine => Self::Headers,
            Self::Headers => Self::Body,
            Self::Body => Self::Done,
            Self::Done => unreachable!(),
        }
    }

    fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

#[derive(Debug, Clone)]
pub struct HttpVersion {
    inner: String,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    method: Method,
    path: String,
    version: HttpVersion,
    headers: Vec<(String, Vec<u8>)>,
}

impl HttpRequest {
    pub async fn parse(input: &mut (impl AsyncRead + Unpin)) -> std::io::Result<HttpRequest> {
        // TODO: Don't use unwrap

        let mut state = RequestParseState::StatusLine;
        let mut reader = BufReader::with_capacity(4096, input);
        let mut buf = Vec::new();
        let mut status_line: (Method, String, HttpVersion) = (
            Method::GET,
            String::new(),
            HttpVersion {
                inner: String::new(),
            },
        );
        let mut headers = Vec::<(String, Vec<u8>)>::new();

        loop {
            // PERF: Replace with zero-alloc parser
            let line_len = reader.read_until(b'\n', &mut buf).await?;
            // SPEC: Do we need to check if it is 0 length?

            let mut line = &buf[..line_len];
            if !line.ends_with(b"\r\n") {
                // Should be an error
                panic!("invalid request header")
            }
            line = &line[..line.len() - 2];
            if line.is_empty() {
                state = match state {
                    // TODO: We need to look at spec for body length detemrination
                    // RequestParseState::Headers => RequestParseState::Body,
                    RequestParseState::Headers | 
                    RequestParseState::Body => {
                        return Ok(Self {
                            method: status_line.0,
                            path: status_line.1,
                            version: status_line.2,
                            headers,
                        });
                    }
                    _ => unreachable!(),
                };
            } else {
                match state {
                    RequestParseState::StatusLine => {
                        // Status line is UTF-8/Ascii compatible
                        let (method, line) =
                            std::str::from_utf8(line).unwrap().split_once(' ').unwrap();
                        let (path, version) = line.split_once(' ').unwrap();
                        status_line.0 = Method::from(method);
                        status_line.1 = path.to_string();
                        status_line.2 = HttpVersion {
                            inner: version.to_string(),
                        };
                        state = RequestParseState::Headers;
                    }
                    RequestParseState::Headers => {
                        let delim = line.iter().position(|b| *b == b':').unwrap();
                        let key = std::str::from_utf8(&line[..delim]).unwrap().to_string();
                        let value = Vec::from(line[delim + 1..].trim_ascii());
                        headers.push((key, value));
                    }
                    RequestParseState::Body => {
                    }
                    RequestParseState::Done => unreachable!(),
                }
            }
            buf.clear();
        }
    }
}
