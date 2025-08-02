use std::collections::{HashMap, hash_map::Entry};

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
}
