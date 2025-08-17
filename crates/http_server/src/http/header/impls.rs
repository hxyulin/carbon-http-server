use std::{fmt, num::ParseIntError};

use crate::http::{header::{HeaderName, Builtin}, uri::{MalformedUriError, UriHost, UriPort}};
use bytes::Bytes;
use uhsapi::ascii::{AsciiStr, InvalidAsciiError};

use super::HeaderValue;

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
    const NAME: HeaderName;
    type Output: HeaderValueTrait;

    fn parse(value: &HeaderValue) -> Result<Self::Output, HeaderParseError> {
        Self::Output::from_header_value(value)
    }
}

pub trait HeaderValueTrait: Sized {
    fn from_header_value(value: &HeaderValue) -> Result<Self, HeaderParseError>;
    fn to_header_value(self, value: &mut HeaderValue);
}

macro_rules! header_struct {
    ($name: ident, $matcher: expr, $ty: ty) => {
        pub struct $name;

        impl HeaderField for $name {
            const IDENT: &'static AsciiStr = unsafe { AsciiStr::from_ascii_unchecked($matcher) };
            const NAME: HeaderName = HeaderName::builtin(Builtin::$name);
            type Output = $ty;
        }
    };
}

/// A Host With Port, for the Host header
/// SPEC: 7.2. Host and :authority
/// ABNF: Host = uri-host [ ":" port ]
#[derive(Debug, Clone)]
pub struct HostWithPort {
    /// The Host
    /// See [`UriHost`]
    pub host: UriHost,
    /// The Port
    /// See [`UriPort`]
    pub port: Option<UriPort>,
}

impl HeaderValueTrait for HostWithPort {
    fn from_header_value(value: &HeaderValue) -> Result<Self, HeaderParseError> {
        if value.len() != 1 {
            todo!("handle error")
        }
        let s = std::str::from_utf8(&value[0]).map_err(|_| InvalidAsciiError)?;

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

    fn to_header_value(self, value: &mut HeaderValue) {
        todo!()
    }
}

impl HeaderValueTrait for u64 {
    fn from_header_value(value: &HeaderValue) -> Result<Self, HeaderParseError> {
        if value.len() != 1 {
            todo!("handle error");
        }
        let s = std::str::from_utf8(&value[0]).map_err(|_| InvalidAsciiError)?;
        Ok(s.parse()?)
    }

    fn to_header_value(self, value: &mut HeaderValue) {
        if !value.is_empty() {
            todo!("content-length should not be set");
        }
        value.push(Bytes::from(self.to_string()));
    }
}

pub enum EncodingKind {}

impl HeaderValueTrait for EncodingKind {
    fn from_header_value(value: &HeaderValue) -> Result<Self, HeaderParseError> {
        todo!()
    }

    fn to_header_value(self, value: &mut HeaderValue) {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionType {
    ProxyConnection,
    KeepAlive,
    TE,
    TransferEncoding,
    Upgrade,
    Close,
    Unknown(Bytes),
}

impl ConnectionType {
    const MAP: &[(&'static [u8], ConnectionType)] = &[
        (b"Proxy-Connection", Self::ProxyConnection),
        (b"Keep-Alive", Self::KeepAlive),
        (b"TE", Self::TE),
        (b"Transfer-Encoding", Self::TransferEncoding),
        (b"Upgrade", Self::Upgrade),
        (b"Close", Self::Close),
    ];
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProxyConnection => f.write_str("Proxy-Connection"),
            Self::KeepAlive => f.write_str("Keep-Alive"),
            Self::TE => f.write_str("TE"),
            Self::TransferEncoding => f.write_str("Transfer-Encoding"),
            Self::Upgrade => f.write_str("Upgrade"),
            Self::Close => f.write_str("Close"),
            Self::Unknown(bytes) => {
                f.write_str(std::str::from_utf8(&bytes).expect("should be valid ascii"))
            }
        }
    }
}

impl HeaderValueTrait for ConnectionType {
    fn from_header_value(value: &HeaderValue) -> Result<Self, HeaderParseError> {
        if value.len() != 1 {
            todo!("handle err");
        }
        let val = &value[0];
        for (str, ty) in Self::MAP {
            if val.eq_ignore_ascii_case(str) {
                return Ok(ty.clone());
            }
        }
        Ok(Self::Unknown(val.clone()))
    }

    fn to_header_value(self, value: &mut HeaderValue) {
        value.push(Bytes::from(self.to_string()));
    }
}

header_struct!(Host, b"host", HostWithPort);
header_struct!(ContentLength, b"content-length", u64);
header_struct!(TransferEncoding, b"transfer-encoding", EncodingKind);
header_struct!(Connection, b"connection", ConnectionType);
