use std::net::SocketAddr;

mod line;
use bytes::Bytes;
pub use line::*;

use crate::http::{header::HeaderMap, method::Method, Body, HttpVersion};

#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub(crate) target: Bytes,
    pub version: HttpVersion,
    pub headers: HeaderMap,
    pub body: Body,
    pub remote: Option<SocketAddr>,
}

impl Request {
    pub fn target(&self) -> Result<RequestTarget, RequestTargetParseError> {
        RequestTarget::try_from(&self.target)
    }
}
