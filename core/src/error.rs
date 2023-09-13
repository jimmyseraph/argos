use std::fmt::Display;

use crate::request::HeaderMap;

pub type Error = hyper::Error;

#[derive(Clone, Debug)]
pub struct ReturnError<B: Display> 
{
    pub response_code: u16,
    pub response_body: B,
    pub headers: HeaderMap,
}

impl <B: Display> ReturnError<B> {
    pub fn new(response_code: u16, response_body: B) -> Self {
        Self {
            response_code,
            response_body,
            headers: HeaderMap::new(),
        }
    }
}