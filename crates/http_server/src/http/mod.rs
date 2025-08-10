pub mod request;
pub mod response;
pub mod header;
pub mod method;
pub mod uri;

pub mod parser;

mod version;
pub use version::{HttpVersion, ParseHttpVersionError};

#[derive(Debug, Clone)]
enum Body {
    None,
    Full(bytes::Bytes),
}
