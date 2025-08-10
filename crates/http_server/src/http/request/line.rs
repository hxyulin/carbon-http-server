use std::{num::NonZeroUsize, str::FromStr};

use bytes::Bytes;
use uhsapi::ascii::{AsciiStr, InvalidAsciiError};

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

impl RequestTarget {
    pub fn as_str(&self) -> &str {
        todo!()
    }
}

pub struct OriginForm {
    data: Bytes,
    /// The starting index of the query (index of question mark)
    /// We can use a NonZeroUsize here because OriginForm starts with a leading slash,
    /// so the question mark can never be at index 0
    query: Option<NonZeroUsize>,
}

impl OriginForm {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, InvalidAsciiError> {
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
            data: Bytes::copy_from_slice(bytes),
            query,
        })
    }
}

impl OriginForm {
    /// Converts to a string, this function does not decode the string
    pub fn as_str(&self) -> &str {
        // SAFETY: This is guaranteed to be ASCII, and should be checked
        unsafe { std::str::from_utf8_unchecked(&self.data) }
    }
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
