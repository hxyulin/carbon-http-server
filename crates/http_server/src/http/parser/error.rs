use std::fmt::Display;

#[derive(Debug, Clone, Copy)]
pub enum Location {
    StartLine,
    Headers,
    Body,
    Trailers,
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

#[derive(Debug)]
#[non_exhaustive]
pub enum ParseErrorKind {
    // Syntax/tokenization
    InvalidMethod,
    InvalidTarget,             // origin-form etc.
    InvalidVersion,
    MalformedHeaderLine,       // no colon / bad OWS
    InvalidHeaderName,         // non-tchar
    InvalidHeaderValue,        // illegal bytes (bare CR/LF)
    UnexpectedByte { expected: u8, found: u8 },

    // Framing
    ConflictingContentLength,
    InvalidContentLength,
    InvalidTransferEncoding,
    ChunkSizeInvalid,
    ChunkCrlfMissing,
    ChunkExtensionsInvalid,

    // Limits
    TooLarge { what: LimitKind, limit: usize, actual: usize },

    // Flow / I/O
    IncompleteMessage,         // ran out before CRLF or before body finished
    Timeout,
    Io(std::io::ErrorKind),

    // Version/feature policy
    VersionNotSupported,       // e.g., HTTP/2 preface on H1
    UnsupportedFeature,        // generic policy gate
}

#[derive(Debug)]
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
        todo!()
    }
}

impl std::error::Error for HttpParseError {}
