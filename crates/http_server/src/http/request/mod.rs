use std::net::SocketAddr;

mod line;
use bytes::Bytes;
pub use line::*;

use crate::http::{header::HeaderMap, method::Method, Body, HttpVersion};

#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub target: Bytes,
    pub version: HttpVersion,
    pub headers: HeaderMap,
    pub body: Body,
    pub remote: Option<SocketAddr>,
}
