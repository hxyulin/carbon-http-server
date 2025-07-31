use std::str::FromStr;

use crate::ascii::AsciiString;

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl<'a, T> From<T> for Method 
    where T: AsRef<str>
{
    fn from(value: T) -> Self {
        match value.as_ref() {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderType {
    Extension(AsciiString),
}

#[derive(Debug, Clone)]
pub struct Header {
    pub ty: HeaderType,
    pub val: Vec<String>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    HTTP_0_9,
    HTTP_1_0,
    HTTP_1_1,
    HTTP_2,
    HTTP_3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidHttpVersion;

impl std::fmt::Display for InvalidHttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid HTTP version")
    }
}

impl std::error::Error for InvalidHttpVersion {}

impl FromStr for HttpVersion {
    type Err = InvalidHttpVersion;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "HTTP/0.9" => Self::HTTP_0_9,
            "HTTP/1.0" => Self::HTTP_1_0,
            "HTTP/1.1" => Self::HTTP_1_1,
            "HTTP/2" => Self::HTTP_2,
            "HTTP/3" => Self::HTTP_3,
            _ => return Err(InvalidHttpVersion),
        })
    }
}

impl HttpVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::HTTP_0_9 => "HTTP/0.9",
            Self::HTTP_1_0 => "HTTP/1.0",
            Self::HTTP_1_1 => "HTTP/1.1",
            Self::HTTP_2 => "HTTP/2",
            Self::HTTP_3 => "HTTP/3",
        }
    }
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLine {
    pub method: Method,
    pub path: AsciiString,
    pub version: HttpVersion,
}

#[derive(Debug, Clone)]
pub struct HttpRequest {
    req_line: RequestLine,
    pub headers: Vec<Header>,
}
