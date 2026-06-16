use crate::http::method::Method;
use derive_setters::Setters;

#[derive(Clone, Debug, Setters)]
pub struct Endpoint {
    pub path: String,
    pub query: Vec<(String, String, bool)>,
    pub method: Method,
}

impl Endpoint {
    pub fn new(url: String) -> Endpoint {
        Self {
            path: url,
            query: Default::default(),
            method: Default::default(),
        }
    }
}
