use std::fmt::Display;

use crate::http::response::StatusCode;

#[derive(Debug, Clone, Copy)]
pub enum Location {
    StartLine,
    Headers,
    Body,
    Trailers,
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::StartLine => "start line",
            Self::Headers => "headers",
            Self::Body => "body",
            Self::Trailers => "trailers",
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LimitKind {
    RequestLineBytes,
    HeaderLineBytes,
    HeaderBytesTotal,
    HeaderCount,
    PathBytes,
    QueryBytes,
    BodyBytes,
    ChunkSizeBytes,
    TrailerBytesTotal,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ParseErrorKind {
    // Syntax/tokenization
    InvalidMethod,
    InvalidTarget, // origin-form etc.
    InvalidVersion,
    MalformedHeaderLine, // no colon / bad OWS
    InvalidHeaderName,   // non-tchar
    InvalidHeaderValue,  // illegal bytes (bare CR/LF)
    UnexpectedByte {
        expected: u8,
        found: u8,
    },
    MissingRequiredHeader,
    DuplicateHeader,

    // Framing
    ConflictingContentLength,
    InvalidContentLength,
    InvalidTransferEncoding,
    ChunkSizeInvalid,
    ChunkCrlfMissing,
    ChunkExtensionsInvalid,

    // Limits
    TooLarge {
        what: LimitKind,
        limit: usize,
        actual: usize,
    },

    // Flow / I/O
    IncompleteMessage, // ran out before CRLF or before body finished
    Timeout,
    Io(std::io::ErrorKind),

    // Version/feature policy
    VersionNotSupported, // e.g., HTTP/2 preface on H1
    UnsupportedFeature,  // generic policy gate
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMethod => f.write_str("invalid method"),
            Self::InvalidTarget => f.write_str("invalid target"),
            Self::InvalidVersion => f.write_str("invalid version"),
            Self::MalformedHeaderLine => f.write_str("malformed header"),
            Self::InvalidHeaderName => f.write_str("invalid_header_name"),
            Self::InvalidHeaderValue => f.write_str("invalid header value"),
            Self::UnexpectedByte { expected, found } => {
                write!(f, "expected byte {}, got {}", expected, found)
            }
            Self::MissingRequiredHeader => f.write_str("missing required header"),
                Self::DuplicateHeader => f.write_str("duplicate header"),
            Self::ConflictingContentLength => f.write_str("conflicting content length"),
            Self::InvalidContentLength => f.write_str("invalid content length"),
            Self::InvalidTransferEncoding => f.write_str("invalid transfer encoding"),
            Self::ChunkSizeInvalid => f.write_str("chunk size invalid"),
            Self::ChunkCrlfMissing => f.write_str("chunk crlf missing"),
            Self::ChunkExtensionsInvalid => f.write_str("chunk extensions invalid"),
            Self::TooLarge {
                what,
                limit,
                actual,
            } => {
                // TODO: Use Display for Limit instead of Debug
                write!(
                    f,
                    "limit {:?} exceeded (limit: {}, actual: {})",
                    what, limit, actual
                )
            }

            Self::IncompleteMessage => f.write_str("incomplete message"),
            Self::Timeout => f.write_str("timed out"),
            Self::Io(err) => Display::fmt(&err, f),
            Self::VersionNotSupported => f.write_str("version not supported"),
            Self::UnsupportedFeature => f.write_str("unsupported feature"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpParseError {
    pub kind: ParseErrorKind,
    pub location: Location,
    /// Byte offset into the *current head/body slice* where we noticed the error.
    /// (For StartLine/Headers, make this an offset into the frozen head Bytes.)
    pub offset: usize,
    /// Optional line index to help debugging/logs (you can compute lazily).
    pub line: Option<usize>,
}

impl Display for HttpParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "http parse error: {} while parsing {} at offset {}",
            self.kind, self.location, self.offset
        )?;
        if let Some(line) = self.line {
            write!(f, "(line {})", line)?;
        }
        Ok(())
    }
}

impl std::error::Error for HttpParseError {}

impl HttpParseError {
    pub fn status_code(&self) -> StatusCode {
        match self.kind {
            ParseErrorKind::InvalidMethod
            | ParseErrorKind::InvalidTarget
            | ParseErrorKind::InvalidVersion
            | ParseErrorKind::MalformedHeaderLine
            | ParseErrorKind::InvalidHeaderName
            | ParseErrorKind::InvalidHeaderValue
            | ParseErrorKind::UnexpectedByte { .. }
            | ParseErrorKind::MissingRequiredHeader
            | ParseErrorKind::DuplicateHeader
            | ParseErrorKind::ConflictingContentLength
            | ParseErrorKind::InvalidContentLength
            | ParseErrorKind::InvalidTransferEncoding
            | ParseErrorKind::ChunkSizeInvalid
            | ParseErrorKind::ChunkCrlfMissing
            | ParseErrorKind::ChunkExtensionsInvalid => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
