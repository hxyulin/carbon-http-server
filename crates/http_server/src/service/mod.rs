use crate::http::{request::Request, response::Response};

pub trait Service: Send + Sync + 'static {
}
