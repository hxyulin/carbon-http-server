use bytes::{Bytes, BytesMut};
use smallvec::SmallVec;
use std::{fmt, ops::Index};
use uhsapi::ascii::{InvalidAsciiError, bytes_are_ascii};

pub use {impls::*, map::*};

mod impls;
mod map;

/// Header Name
/// SPEC: RFC 9110 - 5.1 Field Names
/// OBNF: field-name = token
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(Repr);

impl HeaderName {
    pub const fn builtin(builtin: Builtin) -> Self {
        Self(Repr::Builtin(builtin))
    }
}

impl TryFrom<&Bytes> for HeaderName {
    type Error = InvalidAsciiError;

    fn try_from(bytes: &Bytes) -> Result<Self, Self::Error> {
        Ok(match Builtin::from_bytes(&bytes) {
            Some(builtin) => Self(Repr::Builtin(builtin)),
            None => {
                bytes_are_ascii(bytes)?;
                Self(Repr::Custom(Custom::new(bytes.clone())))
            }
        })
    }
}

impl fmt::Display for HeaderName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Repr::Builtin(builtin) => fmt::Display::fmt(&builtin, f),
            Repr::Custom(bytes) => f.write_str(std::str::from_utf8(&bytes.value).unwrap()),
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
    value: Bytes,
}

impl Custom {
    pub fn new(value: Bytes) -> Self {
        Self { value }
    }
}

impl fmt::Display for Custom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // SAFETY: It should be checked ASCII before being stored
        fmt::Display::fmt(unsafe { std::str::from_utf8_unchecked(&self.value) }, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    Host,
    Connection,
    ContentLength,
    TransferEncoding,
    SetCookie,
    ContentLocation,
    ContentType,
    Date,
    Trailer,
}

impl fmt::Display for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Host => "Host",
            Self::Connection => "Connection",
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

impl Builtin {
    pub fn from_bytes(bytes: &Bytes) -> Option<Self> {
        const MAP: &[(&'static [u8], Builtin)] = &[
            (b"Host", Builtin::Host),
            (b"Connection", Builtin::Connection),
            (b"Content-Length", Builtin::ContentLength),
            (b"Transfer-Encoding", Builtin::TransferEncoding),
            (b"Set-Cookie", Builtin::SetCookie),
            (b"Content-Location", Builtin::ContentLocation),
            (b"Content-Type", Builtin::ContentType),
            (b"Date", Builtin::Date),
            (b"Trailer", Builtin::Trailer),
        ];
        for (name, ty) in MAP {
            if bytes.eq_ignore_ascii_case(name) {
                return Some(*ty);
            }
        }
        None
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

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn collect(&self) -> Bytes {
        let mut bytes = BytesMut::new();
        for val in self.as_slice() {
            bytes.extend_from_slice(val);
            bytes.extend_from_slice(b", ");
        }
        drop(bytes.split_off(bytes.len() - 2));
        bytes.freeze()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Bytes> {
        self.values.iter()
    }
}

impl Index<usize> for HeaderValue {
    type Output = Bytes;
    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

#[cfg(test)]
mod tests {}
