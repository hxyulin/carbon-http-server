use std::fmt::Display;

use bytes::Bytes;
mod builder;
pub use builder::ResponseBuilder;

use crate::http::{Body, HttpVersion, header::HeaderMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusCode(u16);

impl StatusCode {
    pub const OK: Self = Self(200);
    pub const NOT_FOUND: Self = Self(404);
    pub const INTERNAL_SERVER_ERROR: Self = Self(500);

    pub const fn canonical_reason(&self) -> Option<&'static str> {
        Some(match self.0 {
            200 => "OK",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => return None,
        })
    }
}

impl Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub version: HttpVersion,
    pub status: StatusCode,
    pub message: Bytes,
    pub headers: HeaderMap,
    pub body: Body,
}

impl Response {}
