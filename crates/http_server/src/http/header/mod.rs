mod map;
use std::str::FromStr;

use bytes::Bytes;
pub use map::*;
use smallvec::SmallVec;
use uhsapi::ascii::{AsciiStr, AsciiString};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(Repr);

impl From<&AsciiStr> for HeaderName {
    fn from(value: &AsciiStr) -> Self {
        match BuiltinHeader::from_str(value.as_str()) {
            Ok(builtin) => Self(Repr::Builtin(builtin)),
            Err(_) => Self(Repr::Custom(Custom::new(value.to_ascii_string()))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Repr {
    Builtin(BuiltinHeader),
    Custom(Custom),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Custom {
    value: AsciiString,
}

impl Custom {
    pub fn new(value: AsciiString) -> Self {
        Self {
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum BuiltinHeader {}

impl FromStr for BuiltinHeader {
    type Err = ();

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        // TODO: Add builtin types and handle
        Err(())
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
}
