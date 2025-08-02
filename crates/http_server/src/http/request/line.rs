use std::str::FromStr;

use crate::http::method::Method;

// TODO: This is not safe or recommended, replace each of these fields with URI, URL, or other
// structs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTarget {
    /// An Origin request as an URI
    Origin(String),
    /// An abslute URL
    Absolute(String),
    /// An Authority form using URI-host:port format
    Authority(String),
    Asterisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestTargetParseError;

impl TryFrom<&[u8]> for RequestTarget {
    type Error = RequestTargetParseError;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        Ok(match s {
            b"*" => Self::Asterisk,
            other => Self::Origin(std::str::from_utf8(other).unwrap().to_string()),
        })
    }
}

impl std::fmt::Display for RequestTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Origin(s) => f.write_str(s.as_str()),
            Self::Asterisk => f.write_str("*"),
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpVersion {
    pub major: u8,
    pub minor: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseHttpVersionError;

impl std::fmt::Display for ParseHttpVersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid HTTP version")
    }
}

impl std::error::Error for ParseHttpVersionError {}

impl FromStr for HttpVersion {
    type Err = ParseHttpVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(s) = s.strip_prefix("HTTP/") {
            let mut parts = s.splitn(2, '.');
            let major = parts
                .next()
                .ok_or(ParseHttpVersionError)?
                .parse::<u8>()
                .map_err(|_| ParseHttpVersionError)?;
            let minor = parts
                .next()
                .ok_or(ParseHttpVersionError)?
                .parse::<u8>()
                .map_err(|_| ParseHttpVersionError)?;
            Ok(HttpVersion { major, minor })
        } else {
            Err(ParseHttpVersionError)
        }
    }
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP/{}.{}", self.major, self.minor)
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLine {
    pub method: Method,
    pub target: RequestTarget,
    pub version: HttpVersion,
}
