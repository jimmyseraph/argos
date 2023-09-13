use std::sync::RwLock;
use std::{collections::HashMap, fmt::Display, hash::Hash};

use error::ReturnError;
use lazy_static::lazy_static;
use bytes::Bytes;

use http_body_util::Full;

use crate::request::HttpRequest;

pub mod request;
pub mod response;
pub mod server;
pub mod support;
pub mod error;
pub mod util;

pub struct Path {
    inner: String,
}

impl Path {
    pub fn new(path: &str) -> Self {
        Self {
            inner: path.to_string(),
        }
    }

    pub fn pattern_match(&self, other: &String) -> Option<HashMap<String, String>> {
        let mut path_params = HashMap::new();
        let mut iter1 = self.inner.split("/");
        let mut iter2 = other.split("/");
        loop {
            let seg1 = iter1.next();
            let seg2 = iter2.next();
            if seg1.is_none() && seg2.is_none() {
                break;
            }
            if seg1.is_none() || seg2.is_none() {
                return None;
            }
            let seg1 = seg1.unwrap();
            let seg2 = seg2.unwrap();
            if seg1.starts_with(":") {
                path_params.insert(seg1[1..].to_string(), seg2.to_string());
            } else if seg1 != seg2 {
                return None;
            }
        }
        Some(path_params)
    }
}

impl Eq for Path {}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Hash for Path {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

// #[derive(Debug, PartialEq, Clone, Copy)]
// pub enum Formatter {
//     JSON,
//     TEXT,
//     HTML,
// }

// impl Display for Formatter {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Formatter::JSON => write!(f, "json"),
//             Formatter::TEXT => write!(f, "text"),
//             Formatter::HTML => write!(f, "html"),
//         }
//     }
// }

pub struct RouteInfo {
    method: String,
    path: Path,
    handler: Box<dyn Fn(HttpRequest) -> 
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<hyper::Response<Full<Bytes>>, hyper::Error>> + Send>> + Send + Sync>,
}

impl  RouteInfo {
    pub fn new(method: String, path: Path, handler: Box<dyn Fn(HttpRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<hyper::Response<Full<Bytes>>, hyper::Error>> + Send>> + Send + Sync>) -> Self {
        Self {
            method,
            path,
            handler,
        }
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn handler(&self) -> &Box<dyn Fn(HttpRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<hyper::Response<Full<Bytes>>, hyper::Error>> + Send>> + Send + Sync> {
        &self.handler
    }
}

pub enum Chain {
    Continune(HttpRequest),
    Reject(ReturnError<String>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Predicate {
    PathPattern(String),
}

impl Predicate {
    pub fn from_str(t: &str, exp: &str) -> Self {
        match t {
            "path_pattern" => Self::PathPattern(exp.to_string()),
            _ => panic!("unsupported predicate type: {}", t),
        }
    }
}
    
pub struct FilterInfo {
    predicate: Predicate,
    order: u32,
    handler: Box<dyn Fn(HttpRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Chain> + Send>> +Send + Sync>,
} 

impl FilterInfo {
    pub fn new(
        predicate: Predicate, 
        order: u32, 
        handler: Box<dyn Fn(HttpRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Chain> + Send>> +Send + Sync>,
    ) -> Self {
        Self {
            predicate,
            order,
            handler,
        }
    }

    pub fn predicate(&self) -> &Predicate {
        &self.predicate
    }

    pub fn order(&self) -> u32 {
        self.order
    }

    pub fn handler(&self) -> &Box<dyn Fn(HttpRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Chain> + Send>> +Send + Sync> {
        &self.handler
    }
}

lazy_static! {
    pub static ref ROUTE_TABLE: RwLock<Vec<RouteInfo>> = RwLock::new(Vec::new());
    pub static ref FILTER_TABLE: RwLock<Vec<FilterInfo>> = RwLock::new(Vec::new());
}

