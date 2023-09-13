
// pub struct HttpRequest {
//     inner: hyper::Request<hyper::body::Incoming>,
// }

use std::{collections::HashMap, fmt::Debug};

// pub type HttpRequest = hyper::Request<hyper::body::Incoming>;
pub struct HttpRequest {
    inner: hyper::Request<hyper::body::Incoming>,
    path_params: HashMap<String, String>,
    attributes: HashMap<String, String>,
}
pub type Method = hyper::Method;
pub type HeaderMap = hyper::HeaderMap<HeaderValue>;
pub type HeaderValue = hyper::header::HeaderValue;
pub type Version = hyper::Version;
pub type Body = hyper::body::Incoming;

// pub trait RequestExt {
//     fn url_params(&self) -> HashMap<String, String>;
// }

impl HttpRequest {

    pub fn new(req: hyper::Request<hyper::body::Incoming>) -> Self {
        Self {
            inner: req,
            path_params: HashMap::new(),
            attributes: HashMap::new(),
        }
    }

    pub fn set_path_params(&mut self, path_params: HashMap<String, String>) {
        self.path_params = path_params;
    }

    pub fn method(&self) -> &Method {
        self.inner.method()
    }

    pub fn method_mut(&mut self) -> &mut Method {
        self.inner.method_mut()
    }

    pub fn path(&self) -> &str {
        self.inner.uri().path()
    }

    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    pub fn path_params(&self) -> &HashMap<String, String> {
        &self.path_params
    }

    pub fn url_params(&self) -> HashMap<String, String> {
        self.inner.uri().query().unwrap_or("").split("&").map(|kv| {
            let mut iter = kv.split("=");
            let key = iter.next().unwrap_or("");
            let value = iter.next().unwrap_or("");
            (key.to_string(), value.to_string())
        }).collect()
    }

    pub fn attributes(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    pub fn attributes_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.attributes
    }
}

impl Debug for HttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpRequest")
            .field("method", &self.method())
            .field("path", &self.path())
            .field("headers", &self.headers())
            .field("path_params", &self.path_params())
            .field("url_params", &self.url_params())
            .field("attributes", &self.attributes())
            .finish()
    }
}