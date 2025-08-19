pub mod request;
pub mod response;
pub mod header;
pub mod method;
pub mod uri;

pub mod parser;

mod version;
pub use version::{HttpVersion, ParseHttpVersionError};

/// Message Body
/// SPEC: RFC 9112 - 6. Message Body
/// OBNF: message-body = *OCTET
#[derive(Debug, Clone)]
pub enum Body {
    None,
    Full(bytes::Bytes),
}
