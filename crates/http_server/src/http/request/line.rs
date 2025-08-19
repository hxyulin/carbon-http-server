use std::num::NonZeroUsize;

use bytes::Bytes;
use uhsapi::ascii::{AsciiStr, InvalidAsciiError};

use crate::http::uri::{UrlDecodeError, url_decode};

/// A Target for a HTTP Request
/// SPEC: RFC 9112 - 3.2. Request Target
/// ABNF: request-target = origin-form / absolute-form / authority-form / asterisk-form
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTarget {
    /// An Origin request as an URI
    Origin(OriginForm),
    /// An abslute URL
    Absolute(String),
    /// An Authority form using URI-host:port format
    Authority(String),
    /// Asterik Form of a Request Target
    /// SPEC: RFC 9112 - 3.2.4. asterisk-form
    /// ABNF: asterik-form = "*"
    Asterisk,
}

impl RequestTarget {
    pub fn as_str(&self) -> &str {
        // FIXME: This is not only a security vulnerability, but also it doesn't URL decode
        // We should provide the users a Heap Allocated Decoded string / URI components
        match self {
            Self::Asterisk => "*",
            Self::Origin(origin) => origin.as_str(),
            _ => unimplemented!(),
        }
    }
}

/// Origin Form for a Request Target
/// SPEC: RFC 9112 - 3.2.1. origin-form
/// ABNF: origin-form = absolute-path [ "?" query ]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginForm {
    data: Bytes,
    /// The starting index of the query (index of question mark)
    /// We can use a NonZeroUsize here because OriginForm starts with a leading slash,
    /// so the question mark can never be at index 0
    query: Option<NonZeroUsize>,
}

impl OriginForm {
    pub fn from_bytes(bytes: &Bytes) -> Result<Self, InvalidAsciiError> {
        // Check to make sure it is valid ascii
        _ = AsciiStr::from_ascii(bytes)?;
        if *bytes.get(0).unwrap() != b'/' {
            todo!()
        }
        // SAFETY: We checked that byte position 0 is a slash, so it can never be a question mark
        let query = bytes
            .iter()
            .position(|b| *b == b'?')
            .map(|idx| unsafe { NonZeroUsize::new_unchecked(idx) });
        Ok(Self {
            data: bytes.clone(),
            query,
        })
    }

    pub fn path(&self) -> Result<String, UrlDecodeError> {
        // FIXME: Untested
        let component = match self.query {
            Some(query) => &self.data[..query.get()],
            None => &self.data,
        };
        url_decode(component)
    }

    pub fn query(&self) -> Result<Option<String>, UrlDecodeError> {
        // FIXME: Untested
        match self.query {
            Some(query) => Ok(Some(url_decode(&self.data[query.get()..]).unwrap())),
            None => Ok(None),
        }
    }

    /// Converts to a string, this function does not decode the string
    pub fn as_str(&self) -> &str {
        // SAFETY: This is guaranteed to be ASCII, and should be checked
        unsafe { std::str::from_utf8_unchecked(&self.data) }
    }
}

/// Absolute Form of a Request Target
/// SPEC: RFC 9112 - 3.2.2. absolute-form
/// ABNF: absolute-form  = absolute-URI
pub struct AbsoluteForm {}

/// Authority Form of a Request Target
/// SPEC: RFC 9112 - 3.2.3. authority-form
/// ABNF: authority-form = uri-host ":" port
pub struct AuthorityForm {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestTargetParseError;

impl TryFrom<&Bytes> for RequestTarget {
    type Error = RequestTargetParseError;

    fn try_from(s: &Bytes) -> Result<Self, Self::Error> {
        if let Some(fc) = s.get(0).copied() {
            match fc {
                b'*' => {
                    if s.len() > 1 {
                        todo!("handle error")
                    }
                    return Ok(Self::Asterisk);
                }
                b'/' => return Ok(Self::Origin(OriginForm::from_bytes(s).unwrap())),
                _ => {
                    // so it can either be absolute path or authority-form
                    todo!()
                }
            }
        }
        todo!("error, cannot be empty")
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
