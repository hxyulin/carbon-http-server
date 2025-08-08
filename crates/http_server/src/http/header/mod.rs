mod map;
use std::{fmt::Display, num::ParseIntError, str::FromStr};

use bytes::Bytes;
pub use map::*;
use smallvec::SmallVec;
use uhsapi::ascii::{AsciiStr, AsciiString, InvalidAsciiError};

use crate::http::uri::{MalformedUriError, UriHost, UriPort};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(Repr);

impl From<&AsciiStr> for HeaderName {
    fn from(value: &AsciiStr) -> Self {
        match Builtin::from_str(&value.as_str().to_ascii_lowercase()) {
            Ok(builtin) => Self(Repr::Builtin(builtin)),
            Err(_) => Self(Repr::Custom(Custom::new(value.to_ascii_string()))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Repr {
    Builtin(Builtin),
    Custom(Custom),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Custom {
    value: AsciiString,
}

impl Custom {
    pub fn new(value: AsciiString) -> Self {
        Self { value }
    }
}

impl Display for Custom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.value, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Builtin {
    Host,
    ContentLength,
    TransferEncoding,
    SetCookie,
    ContentLocation,
    ContentType,
    Date,
    Trailer,
}

impl Display for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Host => "Host",
            Self::ContentLength => "Content-Length",
            Self::TransferEncoding => "Transfer-Encoding",
            Self::SetCookie => "Set-Cookie",
            Self::ContentLocation => "Content-Location",
            Self::ContentType => "Content-Type",
            Self::Date => "Date",
            Self::Trailer => "Trailer",
        })
    }
}

impl FromStr for Builtin {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "host" => Self::Host,
            "content-length" => Self::ContentLength,
            "transfer-encoding" => Self::TransferEncoding,
            "set-cookie" => Self::SetCookie,
            "content-location" => Self::ContentLocation,
            "content-type" => Self::ContentType,
            "date" => Self::Date,
            "trailer" => Self::Trailer,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct HeaderValue {
    values: SmallVec<[Bytes; 1]>,
}

impl Default for HeaderValue {
    fn default() -> Self {
        Self::new()
    }
}

impl HeaderValue {
    pub fn new() -> Self {
        Self {
            values: SmallVec::new(),
        }
    }

    pub fn push(&mut self, bytes: Bytes) {
        self.values.push(bytes);
    }

    pub fn as_slice(&self) -> &[Bytes] {
        self.values.as_slice()
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum HeaderParseError {
    #[error(transparent)]
    InvalidUri(#[from] MalformedUriError),
    #[error(transparent)]
    InvalidInt(#[from] ParseIntError),
    #[error(transparent)]
    InvalidAscii(#[from] InvalidAsciiError),
}

pub trait HeaderField {
    const IDENT: &'static AsciiStr;
    type Output: FromHeaderValue;

    fn parse(bytes: &Bytes) -> Result<Self::Output, HeaderParseError> {
        Self::Output::from_header_value(bytes)
    }
}

pub trait FromHeaderValue: Sized {
    fn from_header_value(bytes: &Bytes) -> Result<Self, HeaderParseError>;
}

macro_rules! header_struct {
    ($name: ident, $matcher: expr, $ty: ty) => {
        pub struct $name;

        impl HeaderField for $name {
            const IDENT: &'static AsciiStr = unsafe { AsciiStr::from_ascii_unchecked($matcher) };
            type Output = $ty;
        }
    };
}

#[derive(Debug, Clone)]
pub struct HostWithPort {
    pub host: UriHost,
    pub port: Option<UriPort>,
}

impl FromHeaderValue for HostWithPort {
    fn from_header_value(bytes: &Bytes) -> Result<Self, HeaderParseError> {
        let s = std::str::from_utf8(bytes).map_err(|_| InvalidAsciiError)?;

        if let Some((host, port)) = s.rsplit_once(':') {
            if port.is_empty() {
                todo!("handle empty port")
            }
            if port.bytes().all(|c| c.is_ascii_digit()) {
                return Ok(Self {
                    host: host.parse()?,
                    port: Some(port.parse()?),
                });
            }
        }
        Ok(Self {
            host: s.parse()?,
            port: None,
        })
    }
}

header_struct!(Host, b"host", HostWithPort);
