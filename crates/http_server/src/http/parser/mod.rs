use std::{
    fmt::{self, Debug},
    ops::{Index, Range, RangeInclusive},
};

use crate::http::{
    Body,
    header::{ContentLength, HeaderMap, HeaderName, TransferEncoding},
    request::Request,
    response::Response,
};

mod error;
mod line;
use bytes::{Bytes, BytesMut};
pub use error::*;
use memchr::{memchr, memchr2};
use smallvec::SmallVec;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn is_tchar(b: u8) -> bool {
    (b'A'..=b'Z').contains(&b)
        || (b'a'..=b'z').contains(&b)
        || (b'0'..=b'9').contains(&b)
        || matches!(
            b,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

struct Reader<T: AsyncReadExt + Unpin> {
    inner: T,
    buf: BytesMut,
    cursor: usize,
}

impl<READER> Reader<READER>
where
    READER: AsyncReadExt + Unpin,
{
    const BUF_SIZE: usize = 8192;

    pub fn new(reader: READER) -> Self {
        Self {
            inner: reader,
            buf: BytesMut::with_capacity(Self::BUF_SIZE),
            cursor: 0,
        }
    }

    async fn read(&mut self) -> std::io::Result<usize> {
        self.buf.reserve(Self::BUF_SIZE);
        self.inner.read_buf(&mut self.buf).await
    }

    fn get_line(&mut self) -> Option<ReaderLine<'_>> {
        if self.cursor > self.buf.len() {
            return None;
        }

        let line_start = self.cursor;
        let nl_rel = memchr(b'\n', &self.buf[line_start..])?;
        let nl = nl_rel + line_start;
        self.cursor = nl + 1;
        let line_end = if nl_rel > 0 && self.buf[nl - 1] == b'\r' {
            nl - 1..=nl
        } else {
            nl..=nl
        };

        Some(ReaderLine {
            buf: &self.buf,
            line_start,
            line_end,
        })
    }
}

impl<T> Index<Range<usize>> for Reader<T>
where
    T: AsyncReadExt + Unpin,
{
    type Output = [u8];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.buf[index]
    }
}

struct ReaderLine<'a> {
    pub buf: &'a BytesMut,
    line_start: usize,
    line_end: RangeInclusive<usize>,
}

impl ReaderLine<'_> {
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    /// Returns the range of the next word (everything before the next space, or the end of the
    /// line), and advances the
    /// start of the line
    pub fn next_word(&mut self) -> Option<Range<usize>> {
        if self.line_start >= *self.line_end.start() {
            return None;
        }

        let start = self.line_start;
        if let Some(sp) = memchr2(b' ', b'\t', self.as_slice()) {
            self.line_start += sp + 1;
            Some(start..start + sp)
        } else {
            let end = *self.line_end.start();
            self.line_start = end;
            Some(start..end)
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf[self.range()]
    }

    pub fn range(&self) -> Range<usize> {
        self.line_start..*self.line_end.start()
    }

    pub fn next(&mut self, byte: u8) -> Option<Range<usize>> {
        if self.line_start >= *self.line_end.start() {
            return None;
        }
        let start = self.line_start;
        let split = memchr(byte, self.as_slice())? + start;
        self.line_start = split + 1;
        Some(start..split)
    }

    pub fn trim(&self) -> Range<usize> {
        const WHITESPACE: &[u8] = b" \t";
        let mut start = self.line_start;
        let mut end = *self.line_end.start();
        while start < end && WHITESPACE.contains(&self.buf[start]) {
            start += 1;
        }

        while end > start && WHITESPACE.contains(&self.buf[end - 1]) {
            end -= 1;
        }

        start..end
    }
}

/// An HTTP Parser which can parse any HTTP message ()
pub struct Parser<READER: AsyncReadExt + Unpin> {
    reader: Reader<READER>,
}

pub type HttpParseResult<T> = Result<T, HttpParseError>;

trait LineParse: Sized {
    type Output;

    fn parse(line: ReaderLine) -> HttpParseResult<Self>;
    fn to_output(
        bytes: Bytes,
        data: Self,
        headers: HeaderMap,
        body: Body,
    ) -> HttpParseResult<Self::Output>;
}

#[derive(Debug)]
struct HeaderIx {
    name: Range<usize>,
    value: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    Line,
    Headers,
    Body,
}

impl Into<Location> for ParseState {
    fn into(self) -> Location {
        match self {
            Self::Line => Location::StartLine,
            Self::Headers => Location::Headers,
            Self::Body => Location::Body,
        }
    }
}

impl<READER> Parser<READER>
where
    READER: AsyncReadExt + Unpin,
{
    pub fn new(reader: READER) -> Self {
        Self {
            reader: Reader::new(reader),
        }
    }

    async fn parse_message<M: LineParse>(&mut self) -> HttpParseResult<M::Output> {
        // Parses an entire HTTP Request Message
        // SPEC: RFC 9112 - 2.1 Message Format
        // ABNF:
        //  HTTP-message = start-line CRLF *( field-line CRLF ) CRLF [ message-body ]
        //  start-line = request-line | status-line

        let mut s_line: Option<M> = None;
        let mut headers = SmallVec::<[HeaderIx; 32]>::new();
        let mut state = ParseState::Line;
        let mut line_cnt = 0;

        // Here we lazily parse the start line and headers
        'outer: loop {
            while let Some(mut line) = self.reader.get_line() {
                line_cnt += 1;
                match state {
                    ParseState::Line => {
                        s_line = Some(M::parse(line)?);
                        state = ParseState::Headers;
                    }
                    ParseState::Headers => {
                        // Header Field Parsing
                        // SPEC: RFC 9112 5 Field Syntax
                        // OBNF: field-line = field-name ":" OWS field-value OWS
                        if line.is_empty() {
                            // We don't parse the body here
                            state = ParseState::Body;
                            break 'outer;
                        }
                        if memchr2(b' ', b'\t', line.as_slice()) == Some(0) {
                            // Starts with space, horizontal tab, do Obsolete Line Folding
                            // SPEC: RFC 9112 - 5.2. Obsolete Line Folding
                            todo!()
                        }

                        let name = line.next(b':').ok_or_else(|| HttpParseError {
                            kind: ParseErrorKind::MalformedHeaderLine,
                            location: state.into(),
                            offset: line.line_start,
                            line: Some(line_cnt),
                        })?;
                        if !line.buf[name.clone()].iter().copied().all(is_tchar) {
                            return Err(HttpParseError {
                                kind: ParseErrorKind::InvalidHeaderName,
                                location: state.into(),
                                offset: name.start,
                                line: Some(line_cnt),
                            });
                        }
                        let value = line.trim();
                        headers.push(HeaderIx { name, value });
                    }
                    ParseState::Body => unreachable!(),
                }

                continue;
            }

            if 0 == self.reader.read().await.unwrap() {
                return Err(HttpParseError {
                    kind: ParseErrorKind::IncompleteMessage,
                    location: state.into(),
                    offset: self.reader.cursor,
                    line: Some(line_cnt),
                });
            }
        }

        let header_bytes = self.reader.buf.split_to(self.reader.cursor).freeze();
        // Reset cursor
        self.reader.cursor = 0;
        let mut header_map = HeaderMap::with_capacity(headers.len());
        for header in headers {
            let name = header_bytes.slice(header.name);
            let value = header_bytes.slice(header.value);
            let name = HeaderName::try_from(&name).map_err(|_| todo!())?;
            header_map.entry(name).push(value);
        }

        // Now we can parse body
        assert_eq!(state, ParseState::Body);
        let body = if let Some(_encoding) = header_map.get_header::<TransferEncoding>().unwrap() {
            todo!()
        } else if let Some(cl) = header_map.get_header::<ContentLength>().unwrap() {
            // TODO: Handle message larger than 4GB on 32bit maybe?
            let cl = cl as usize;
            // Remove all header chunks
            let mut body_buf = self.reader.buf.split_to(cl.min(self.reader.buf.len()));
            let old_len = body_buf.len();
            // We can safety resize, because the size is at most cl
            body_buf.resize(cl, 0);
            self.reader
                .inner
                .read_exact(&mut body_buf[old_len..cl])
                .await
                .unwrap();
            Body::Full(body_buf.freeze())
        } else {
            // Everything else is part of the next request
            Body::None
        };

        M::to_output(
            header_bytes,
            s_line.expect("status line should be parsed"),
            header_map,
            body,
        )
    }

    pub async fn parse_request(&mut self) -> HttpParseResult<Request> {
        self.parse_message::<line::RequestLine>().await
    }

    pub async fn parse_response(&mut self) -> HttpParseResult<Response> {
        self.parse_message::<line::ResponseLine>().await
    }
}

pub struct Sender<WRITER: AsyncWriteExt + Unpin> {
    writer: WRITER,
    buf: BytesMut,
}

impl<WRITER> Sender<WRITER>
where
    WRITER: AsyncWriteExt + Unpin,
{
    pub fn new(writer: WRITER) -> Self {
        Self {
            writer,
            buf: BytesMut::with_capacity(8192),
        }
    }

    async fn send_headers(&mut self, headers: HeaderMap) -> std::io::Result<()> {
        use std::fmt::Write;
        for (name, value) in headers.iter() {
            write!(self, "{}: ", name).unwrap();
            self.buf.extend_from_slice(&value.collect());
            write!(self, "\r\n").unwrap();
        }
        write!(self, "\r\n").unwrap();
        Ok(())
    }

    pub async fn send_request(&mut self, request: Request) -> std::io::Result<()> {
        use std::fmt::Write;
        write!(
            self,
            "{} {} {}\r\n",
            request.method,
            std::str::from_utf8(&request.target).unwrap(),
            request.version
        )
        .unwrap();
        self.send_headers(request.headers).await?;
        match request.body {
            Body::None => {}
            Body::Full(bytes) => self.buf.extend_from_slice(&bytes),
        }
        self.flush().await?;
        Ok(())
    }

    pub async fn send_response(&mut self, response: Response) -> std::io::Result<()> {
        use std::fmt::Write;
        write!(
            self,
            "{} {} {}\r\n",
            response.version,
            response.status,
            std::str::from_utf8(&response.message).unwrap()
        )
        .unwrap();
        self.send_headers(response.headers).await?;
        match response.body {
            Body::None => {}
            Body::Full(bytes) => self.buf.extend_from_slice(&bytes),
        }
        self.flush().await?;
        Ok(())
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        self.writer.write_all(&self.buf).await?;
        self.buf.clear();
        self.writer.flush().await
    }
}

impl<WRITER> fmt::Write for Sender<WRITER>
where
    WRITER: AsyncWriteExt + Unpin,
{
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buf.extend_from_slice(s.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod reader {
        use bytes::BytesMut;

        use crate::http::parser::ReaderLine;

        #[test]
        fn line_next_word() {
            let content = "this is a test a\n";
            let end = content.as_bytes().iter().position(|b| *b == b'\n').unwrap();
            let mut line = ReaderLine {
                buf: &BytesMut::from(content),
                line_start: 0,
                line_end: end..=end,
            };

            assert_eq!(line.next_word(), Some(0..4));
            assert_eq!(line.next_word(), Some(5..7));
            assert_eq!(line.next_word(), Some(8..9));
            assert_eq!(line.next_word(), Some(10..14));
            assert_eq!(line.next_word(), Some(15..16));
            assert_eq!(line.next_word(), None);
        }

        #[test]
        fn line_next() {
            let content = "name: value: a\n";
            let end = content.as_bytes().iter().position(|b| *b == b'\n').unwrap();
            let mut line = ReaderLine {
                buf: &BytesMut::from(content),
                line_start: 0,
                line_end: end..=end,
            };

            assert_eq!(line.next(b':'), Some(0..4));
            assert_eq!(line.as_slice(), b" value: a");
        }

        #[test]
        fn line_trim() {
            let content = " \t\tvalue with spaces\t \t\n";
            let end = content.as_bytes().iter().position(|b| *b == b'\n').unwrap();
            let line = ReaderLine {
                buf: &BytesMut::from(content),
                line_start: 0,
                line_end: end..=end,
            };
            let trimmed = line.trim();
            assert_eq!(trimmed, 3..20);
        }
    }
}
