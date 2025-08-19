use std::collections::{HashMap, hash_map};

use bytes::Bytes;

use crate::http::header::{Builtin, HeaderField, HeaderParseError};

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

    pub fn with_capacity(size: usize) -> Self {
        Self {
            map: HashMap::with_capacity(size),
        }
    }

    pub fn entry(&mut self, name: HeaderName) -> &mut HeaderValue {
        self.map.entry(name).or_insert(HeaderValue::default())
    }

    pub fn contains(&mut self, name: &HeaderName) -> bool {
        self.map.contains_key(&name)
    }

    pub fn get_header<T: HeaderField>(&self) -> Result<Option<T::Output>, HeaderParseError> {
        let name = HeaderName::builtin(
            Builtin::from_bytes(&Bytes::from_static(T::IDENT.as_bytes()))
                .expect("invalid header name"),
        );
        let val = match self.map.get(&name) {
            None => return Ok(None),
            Some(val) => val,
        };
        T::parse(val).map(Some)
    }

    pub fn iter(&self) -> hash_map::Iter<'_, HeaderName, HeaderValue> {
        self.map.iter()
    }
}
