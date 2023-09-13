use std::fmt::Display;
use argos_macros::{route, register};
use serde_json;
use serde::{Deserialize, Serialize};
use argos::{server::Server, request::HttpRequest, error::ReturnError};

#[derive(Serialize, Deserialize)]
pub struct Hello {
    name: String,
    greeting: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MyError {
    code: u32,
    msg: String,
}

impl Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{code:{}, msg:{}}}", self.code, self.msg)
    }
}

#[route(GET, path = "/api/hello", formatter = "json")]
pub fn hello(req: HttpRequest) -> Result<Hello, ReturnError<MyError>> {
    let url_params = req.url_params();
    let name = url_params.get("name");
    // let path_params = req.path_params();
    use std::{thread, time};
    let millis = time::Duration::from_millis(800);
    thread::sleep(millis);
    match name {
        Some(name) => Ok(Hello { name: name.to_string(), greeting: "hello".to_string() }),
        None => Err(
            ReturnError::new(
                400,
                MyError { code: 1, msg: "name is required".to_string() },
            ),
        )
    }
}

#[tokio::main]
async fn main() {
    let server = Server::builder(([127, 0, 0, 1], 3000).into())
        .h2()
        .ssl(
            "core/examples/rsa_private.key", 
            "core/examples/cert.crt", 
            argos::server::SslFiletype::PEM,
        ).unwrap()
        .build()
        .await
        .unwrap();
    server.start().await.unwrap();
}