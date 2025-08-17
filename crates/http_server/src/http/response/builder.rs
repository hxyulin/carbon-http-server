use bytes::Bytes;

use crate::http::{
    Body, HttpVersion,
    header::{ContentLength, HeaderField, HeaderMap, HeaderName, HeaderValueTrait},
    request::Request,
    response::{Response, StatusCode},
};

pub struct ResponseBuilder {
    version: HttpVersion,
    status: StatusCode,
    message: String,
    headers: HeaderMap,
    body: Body,
}

impl ResponseBuilder {
    pub fn from_req(req: &Request, status: StatusCode) -> Self {
        Self::new(req.version, status)
    }

    pub fn new(version: HttpVersion, status: StatusCode) -> Self {
        Self {
            version,
            status,
            message: String::new(),
            headers: HeaderMap::new(),
            body: Body::None,
        }
    }

    pub fn build(self) -> Response {
        let ResponseBuilder {
            version,
            status,
            message,
            headers,
            body,
        } = self;

        let message = if message.is_empty() {
            Bytes::from_static(
                status
                    .canonical_reason()
                    .unwrap_or("Unknown Reason")
                    .as_bytes(),
            )
        } else {
            Bytes::from(message)
        };

        Response {
            version,
            message,
            status,
            headers,
            body,
        }
    }

    pub fn set_header<NAME>(mut self, val: NAME::Output) -> Self
    where
        NAME: HeaderField,
    {
        val.to_header_value(self.headers.entry(NAME::NAME));
        self
    }

    pub fn add_header(mut self, name: &Bytes, val: Bytes) -> Self {
        self.headers
            .entry(HeaderName::try_from(name).expect("header is not valid ascii"))
            .push(val);
        self
    }

    pub fn body(mut self, bytes: Bytes) -> Self {
        let len = bytes.len() as u64;
        self.body = Body::Full(bytes);
        self.set_header::<ContentLength>(len)
    }

    // pub fn body_ext(mut self)
}
