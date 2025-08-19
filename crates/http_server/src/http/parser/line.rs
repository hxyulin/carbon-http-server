use std::ops::Range;

use bytes::Bytes;
use uhsapi::ascii::AsciiStr;

use crate::http::{
    Body, HttpVersion,
    header::{Builtin, HeaderMap, HeaderName},
    method::Method,
    parser::{HttpParseError, HttpParseResult, LineParse, Location, ParseErrorKind},
    request::Request,
    response::Response,
};

/// The Request Line for a HTTP Message
/// SPEC: RFC 9112 - 3. Request Line
/// ABNF: request-line = method SP request-target SP HTTP-version
#[derive(Debug)]
pub struct RequestLine {
    pub method: Range<usize>,
    pub target: Range<usize>,
    pub version: HttpVersion,
}

impl LineParse for RequestLine {
    type Output = Request;

    fn parse(mut line: super::ReaderLine) -> super::HttpParseResult<Self> {
        #[inline]
        fn make_err(line: &super::ReaderLine) -> HttpParseError {
            HttpParseError {
                kind: ParseErrorKind::MalformedHeaderLine,
                location: Location::StartLine,
                offset: line.line_start,
                line: None,
            }
        }
        // SPEC: RFC 9112 3 Request Line
        // OBNF: request-line = method SP request-target SP HTTP-version

        let method = line.next_word().ok_or_else(|| make_err(&line))?;
        let target = line.next_word().ok_or_else(|| make_err(&line))?;
        let version =
            AsciiStr::from_ascii(&line.buf[line.next_word().ok_or_else(|| make_err(&line))?])
                .map_err(|_| make_err(&line))?
                .as_str()
                .parse::<HttpVersion>()
                .map_err(|_| make_err(&line))?;

        if !line.is_empty() {
            return Err(HttpParseError {
                kind: ParseErrorKind::InvalidVersion,
                location: Location::StartLine,
                offset: line.line_start,
                line: None,
            });
        }

        Ok(Self {
            method,
            target,
            version,
        })
    }

    fn to_output(
        bytes: Bytes,
        data: Self,
        mut headers: HeaderMap,
        body: Body,
    ) -> HttpParseResult<Self::Output> {
        if !headers.contains(&HeaderName::builtin(Builtin::Host)) {
            return Err(HttpParseError {
                kind: ParseErrorKind::MissingRequiredHeader,
                location: Location::Headers,
                offset: 0,
                line: None,
            });
        }

        Ok(Self::Output {
            method: Method::try_from(bytes.slice(data.method)).unwrap(),
            target: bytes.slice(data.target),
            version: data.version,
            headers,
            body,
            remote: None,
        })
    }
}

/// The Response Line for a HTTP Message
/// SPEC: RFC 9112 - 3.2. Request Target
/// ABNF:
///     status-line = HTTP-version SP status-code SP [ reason-phrase ]
///     status-code    = 3DIGIT
///     reason-phrase  = 1*( HTAB / SP / VCHAR / obs-text )
#[derive(Debug)]
pub struct ResponseLine {
    pub version: HttpVersion,
    pub status_code: u32,
    pub reason_phrase: Option<Range<usize>>,
}

impl LineParse for ResponseLine {
    type Output = Response;

    fn parse(line: super::ReaderLine) -> super::HttpParseResult<Self> {
        todo!()
    }

    fn to_output(
        bytes: Bytes,
        data: Self,
        headers: HeaderMap,
        body: Body,
    ) -> HttpParseResult<Self::Output> {
        todo!()
    }
}
