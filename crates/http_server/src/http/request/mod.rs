use std::str::FromStr;

use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use uhsapi::ascii::{AsAsciiStr, AsciiStr, InvalidAsciiError};

mod line;
pub use line::*;

use crate::http::{
    header::{HeaderMap, HeaderName},
    method::Method,
};

#[derive(Debug, Clone)]
pub struct HttpRequest {
    req_line: RequestLine,
    headers: HeaderMap,
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

pub struct HttpRequestParser<T: AsyncRead + Unpin> {
    reader: BufReader<T>,
    line_buf: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum RequestParseError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("unexpected EOF")]
    UnexpectedEof,
    #[error("invalid status line")]
    InvalidStatusLine,
    #[error("invalid header")]
    InvalidHeader,
    #[error(transparent)]
    InvalidAscii(#[from] InvalidAsciiError),
    #[error(transparent)]
    InvalidVersion(#[from] ParseHttpVersionError),
}

impl PartialEq for RequestParseError {
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
    async fn read_line(&mut self) -> Result<&[u8], RequestParseError> {
        // TODO: Timeout
        self.line_buf.clear();
        let n = self.reader.read_until(b'\n', &mut self.line_buf).await?;
        if n == 0 {
            return Err(RequestParseError::UnexpectedEof);
        }
        println!("{}", n);
        // We know it ends with \r\n, just truncate
        self.line_buf.truncate(self.line_buf.len() - 2);
        Ok(self.line_buf.as_slice())
    }

    async fn parse_req_line(&mut self) -> Result<RequestLine, RequestParseError> {
        let mut chunks = self.read_line().await?.split(|b| *b == b' ');
        let method = chunks.next().ok_or(RequestParseError::InvalidStatusLine)?;
        let method: Method = method.as_ascii_str()?.into();
        let target: RequestTarget = chunks
            .next()
            .ok_or(RequestParseError::InvalidStatusLine)?
            .try_into()
            .unwrap();
        let version = HttpVersion::from_str(
            chunks
                .next()
                .ok_or(RequestParseError::InvalidStatusLine)?
                .as_ascii_str()?
                .as_str(),
        )?;
        if chunks.next().is_some() {
            return Err(RequestParseError::InvalidStatusLine);
        }

        Ok(RequestLine {
            method,
            target,
            version,
        })
    }

    async fn parse_header(
        &mut self,
        header_map: &mut HeaderMap,
    ) -> Result<Option<()>, RequestParseError> {
        let line = self.read_line().await?;
        // Headers are finished, we return None
        if line.is_empty() {
            return Ok(None);
        }

        let (ty, value) = line
            .split_once(|b| *b == b':')
            .ok_or(RequestParseError::InvalidHeader)?;

        // We don't yet check encoding, we just trim
        let name = HeaderName::from(AsciiStr::from_ascii(ty)?);
        let value = value.trim_ascii();

        header_map.entry(name).push(Bytes::copy_from_slice(value));

        Ok(Some(()))
    }

    async fn parse_headers(&mut self) -> Result<HeaderMap, RequestParseError> {
        let mut header_map = HeaderMap::new();

        while self.parse_header(&mut header_map).await?.is_some() {
            // We might want to do something here?
        }

        Ok(header_map)
    }

    pub async fn parse(input: T) -> Result<HttpRequest, RequestParseError> {
        let mut parser = Self {
            reader: BufReader::new(input),
            line_buf: Vec::new(),
        };

        let req_line = parser.parse_req_line().await?;
        let headers = parser.parse_headers().await?;

        Ok(HttpRequest { req_line, headers })
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::ChannelReader;

    use super::*;
    use std::time::Duration;
    use tokio::{sync::mpsc, time::sleep};

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
        let cases: &[(&'static [u8], Result<RequestLine, RequestParseError>)] = &[
            (
                b"GET / HTTP/1.1\r\n",
                Ok(RequestLine {
                    method: Method::GET,
                    target: RequestTarget::Origin("/".to_string()),
                    version: HttpVersion { major: 1, minor: 1 },
                }),
            ),
            (
                b"PATCH /login?username=xxx123 HTTP/1.1\r\n",
                Ok(RequestLine {
                    method: Method::PATCH,
                    target: RequestTarget::Origin("/login?username=xxx123".to_string()),
                    version: HttpVersion { major: 1, minor: 1 },
                }),
            ),
            (b"", Err(RequestParseError::UnexpectedEof)),
            (
                b"PATCH /etc/shadow HTTP/1.1 something else\r\n",
                Err(RequestParseError::InvalidStatusLine),
            ),
        ];

        for (data, res) in cases {
            let mut parser = setup_parser_with_data(data).await;
            let sl = parser.parse_req_line().await;
            match (sl, res) {
                (Ok(sl), Ok(res)) => assert_eq!(&sl, res),
                (Err(err1), Err(err2)) => assert_eq!(&err1, err2),
                (sl, res) => todo!("test case mismatch"),
            }
        }
    }
}
