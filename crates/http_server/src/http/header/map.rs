use std::collections::HashMap;

use bytes::BytesMut;

use crate::http::header::{HeaderField, HeaderParseError};

use super::{HeaderName, HeaderValue};

#[derive(Debug, Clone)]
pub struct HeaderMap {
    map: HashMap<HeaderName, HeaderValue>,
}

impl HeaderMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn entry(&mut self, name: HeaderName) -> &mut HeaderValue {
        self.map.entry(name).or_insert(HeaderValue::default())
    }

    pub fn get_header<T: HeaderField>(&self) -> Result<Option<T::Output>, HeaderParseError> {
        let val = match self.map.get(&HeaderName::from(T::IDENT)) {
            None => return Ok(None),
            Some(val) => val,
        };
        // TODO: Check header behaviour
        let mut bytes = BytesMut::new();
        for val in val.as_slice() {
            bytes.extend_from_slice(val);
        }
        T::parse(&bytes.freeze()).map(Some)
    }
}
