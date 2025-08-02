use std::fmt::{Debug, Display};

use uhsapi::ascii::{AsciiStr, AsciiString};

/// An HTTP Method
/// SPEC: Defined in RFC9112 3.1
#[derive(Clone, PartialEq, Eq)]
pub struct Method(Repr);

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl Debug for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl From<&AsciiStr> for Method
{
    fn from(value: &AsciiStr) -> Self {
        match Builtin::try_from(value) {
            Ok(builtin) => Method(Repr::Builtin(builtin)),
            Err(_) => Method(Repr::Custom(value.to_ascii_string())),
        }
    }
}

impl Method {
    pub const GET: Self = Self(Repr::Builtin(Builtin::GET));
    pub const POST: Self = Self(Repr::Builtin(Builtin::POST));
    pub const PUT: Self = Self(Repr::Builtin(Builtin::PUT));
    pub const DELETE: Self = Self(Repr::Builtin(Builtin::DELETE));
    pub const PATCH: Self = Self(Repr::Builtin(Builtin::PATCH));
    pub const OPTIONS: Self = Self(Repr::Builtin(Builtin::OPTIONS));
    pub const CONNECT: Self = Self(Repr::Builtin(Builtin::CONNECT));
    pub const TRACE: Self = Self(Repr::Builtin(Builtin::TRACE));
    pub const HEAD: Self = Self(Repr::Builtin(Builtin::HEAD));

    pub const fn custom(str: AsciiString) -> Self {
        Self(Repr::Custom(str))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Repr {
    Builtin(Builtin),
    Custom(AsciiString),
}

impl Display for Repr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin(builtin) => std::fmt::Display::fmt(builtin, f),
            Self::Custom(custom) => std::fmt::Display::fmt(custom, f),
        }
    }
}

// It should be possible for rust to optimize it to 24 bits, we check that it is the case
static_assertions::assert_eq_size!(Repr, AsciiString);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Builtin {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    CONNECT,
    TRACE,
    HEAD,
}

impl Builtin {
    /// Safe Methods are methods which can be cached by
    /// SPEC: [RFC 9110 9.2.1 Safe Methods](https://httpwg.org/specs/rfc9110.html#safe.methods)
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::GET | Self::HEAD | Self::OPTIONS | Self::TRACE)
    }

    /// Idempotent Methods are requests where the side effects are the same if multiple identical
    /// requests are sent
    /// SPEC: [RFC 9110 9.2.2 Idempotent Methods](https://httpwg.org/specs/rfc9110.html#idempotent.methods)
    pub fn is_idempotent(&self) -> bool {
        match self {
            Self::PUT | Self::DELETE => true,
            other => other.is_safe(),
        }
    }
}

impl TryFrom<&AsciiStr> for Builtin {
    type Error = ();

    fn try_from(value: &AsciiStr) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "GET" => Self::GET,
            "POST" => Self::POST,
            "PUT" => Self::PUT,
            "DELETE" => Self::DELETE,
            "PATCH" => Self::PATCH,
            "OPTIONS" => Self::OPTIONS,
            "CONNECT" => Self::CONNECT,
            "TRACE" => Self::TRACE,
            "HEAD" => Self::HEAD,
            _ => return Err(()),
        })
    }
}

impl Display for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::GET => "GET",
            Self::POST => "POST",
            Self::PUT => "PUT",
            Self::DELETE => "DELETE",
            Self::PATCH => "PATCH",
            Self::OPTIONS => "OPTIONS",
            Self::CONNECT => "CONNECT",
            Self::TRACE => "TRACE",
            Self::HEAD => "HEAD",
        })
    }
}
