//! An async HTTP server implementation in rust

#![feature(async_fn_traits)]

pub mod sync;

use std::{net::SocketAddr, str::FromStr, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader},
    net::{TcpSocket, TcpStream},
};
use uhsapi::{
    ascii::{AsAsciiStr, InvalidAsciiError},
    http::{HttpVersion, InvalidHttpVersion, Method, RequestLine},
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
        todo!();
        Ok(())
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

struct HttpRequestParser<T: AsyncRead + Unpin> {
    reader: BufReader<T>,
    line_buf: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum HttpParseError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("unexpected EOF")]
    UnexpectedEof,
    #[error("invalid status line")]
    InvalidStatusLine,
    #[error(transparent)]
    InvalidAscii(#[from] InvalidAsciiError),
    #[error(transparent)]
    InvalidVersion(#[from] InvalidHttpVersion),
}

impl PartialEq for HttpParseError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IoError(_), Self::IoError(_)) => true,
            (Self::UnexpectedEof, Self::UnexpectedEof) => true,
            (Self::InvalidStatusLine, Self::InvalidStatusLine) => true,
            (Self::InvalidAscii(err1), Self::InvalidAscii(err2)) => err1 == err2,
            _ => false,
        }
    }
}

impl<T> HttpRequestParser<T>
where
    T: AsyncRead + Unpin,
{
    /// Reads a line, removing the CRLF from the end
    async fn read_line(&mut self) -> Result<&[u8], HttpParseError> {
        // TODO: Timeout
        self.line_buf.clear();
        let n = self.reader.read_until(b'\n', &mut self.line_buf).await?;
        if n == 0 {
            return Err(HttpParseError::UnexpectedEof);
        }
        // We know it ends with \r\n, just truncate
        self.line_buf.truncate(self.line_buf.len() - 2);
        Ok(self.line_buf.as_slice())
    }

    async fn parse_status_line(&mut self) -> Result<RequestLine, HttpParseError> {
        let mut chunks = self.read_line().await?.split(|b| *b == b' ');
        let method = chunks.next().ok_or(HttpParseError::InvalidStatusLine)?;
        let method: Method = method.as_ascii_str()?.into();
        let path = chunks
            .next()
            .ok_or(HttpParseError::InvalidStatusLine)?
            .as_ascii_str()?
            .to_ascii_string();
        let version = HttpVersion::from_str(
            chunks
                .next()
                .ok_or(HttpParseError::InvalidStatusLine)?
                .as_ascii_str()?
                .as_str(),
        )?;
        if chunks.next().is_some() {
            return Err(HttpParseError::InvalidStatusLine);
        }

        Ok(RequestLine {
            method,
            path,
            version,
        })
    }
}

/*
impl HttpRequest {
    pub async fn parse(input: &mut (impl AsyncRead + Unpin)) -> std::io::Result<HttpRequest> {
        // TODO: Don't use unwrap

        let mut state = RequestParseState::StatusLine;
        let mut reader = BufReader::with_capacity(4096, input);
        let mut buf = Vec::new();
        let mut status_line: Option<RequestLine> = None;
        let mut headers = Vec::<(AsciiString, Vec<u8>)>::new();

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
                    RequestParseState::Headers | RequestParseState::Body => {
                        let status_line = status_line.unwrap();
                        return Ok(Self {
                            method: status_line.method,
                            path: status_line.path,
                            version: status_line.version,
                            headers,
                        });
                    }
                    _ => unreachable!(),
                };
            } else {
                match state {
                    RequestParseState::StatusLine => {
                        todo!();
                        state = RequestParseState::Headers;
                    }
                    RequestParseState::Headers => {
                        let delim = line.iter().position(|b| *b == b':').unwrap();
                        let key = AsciiStr::from_ascii(&line[..delim])
                            .unwrap()
                            .to_ascii_string();
                        let value = Vec::from(line[delim + 1..].trim_ascii());
                        headers.push((key, value));
                    }
                    RequestParseState::Body => {}
                    RequestParseState::Done => unreachable!(),
                }
            }
            buf.clear();
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use crate::sync::ChannelReader;

    use super::*;
    use std::time::Duration;
    use tokio::{sync::mpsc, time::sleep};
    use uhsapi::ascii::IntoAsciiString;

    async fn setup_parser<'a, F>(f: F) -> HttpRequestParser<ChannelReader>
    where
        F: AsyncFnOnce(mpsc::Sender<u8>) + Send + 'static,
        F::CallOnceFuture: Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<u8>(32);

        tokio::spawn(async move {
            f(tx).await;
        });
        HttpRequestParser {
            reader: BufReader::new(ChannelReader::new(rx)),
            line_buf: Vec::new(),
        }
    }

    async fn setup_parser_with_data(data: &[u8]) -> HttpRequestParser<ChannelReader> {
        let data = Vec::from(data);
        setup_parser(|tx: mpsc::Sender<u8>| async move {
            sleep(Duration::from_millis(10)).await;
            for ch in data.iter() {
                tx.send(*ch).await.unwrap();
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
    }

    #[tokio::test]
    async fn test_parser_read_line() {
        const LINE: &'static [u8] = b"GET / HTTP/1.1\r\n";
        let mut parser = setup_parser_with_data(LINE).await;
        let line = parser.read_line().await.unwrap();
        assert_eq!(line, &LINE[..LINE.len() - 2]);
    }

    #[tokio::test]
    async fn test_parser_parse_status_line() {
        let cases: &[(&'static [u8], Result<RequestLine, HttpParseError>)] = &[
            (
                b"GET / HTTP/1.1\r\n",
                Ok(RequestLine {
                    method: Method::GET,
                    path: "/".to_string().into_ascii_string().unwrap(),
                    version: HttpVersion::HTTP_1_1,
                }),
            ),
            (
                b"PATCH /login?username=xxx123 HTTP/1.1\r\n",
                Ok(RequestLine {
                    method: Method::PATCH,
                    path: "/login?username=xxx123"
                        .to_string()
                        .into_ascii_string()
                        .unwrap(),
                    version: HttpVersion::HTTP_1_1,
                }),
            ),
            (b"", Err(HttpParseError::UnexpectedEof)),
            (
                b"PATCH /etc/shadow HTTP/1.1 something else\r\n",
                Err(HttpParseError::InvalidStatusLine),
            ),
        ];

        for (data, res) in cases {
            let mut parser = setup_parser_with_data(data).await;
            let sl = parser.parse_status_line().await;
            match (sl, res) {
                (Ok(sl), Ok(res)) => assert_eq!(&sl, res),
                (Err(err1), Err(err2)) => assert_eq!(&err1, err2),
                (sl, res) => todo!("test case mismatch"),
            }
        }
    }
}
