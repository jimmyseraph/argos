
use argos::{request::HttpRequest, error::ReturnError, server::Server, request::HeaderValue, Chain};
use argos_macros::{route, filter, register};

#[route(GET, path = "/api/hello", formatter = "text")]
pub fn hello(req: HttpRequest) -> Result<String, ReturnError<String>> {
    let url_params = req.url_params();
    let name = url_params.get("name");
    let path_params = req.path_params();
    use std::{thread, time};
    let millis = time::Duration::from_millis(800);
    thread::sleep(millis);
    Ok(format!("hello {:?}, {:?}, {:?}", name, path_params, req.attributes()))
}

#[filter(path_pattern="/api/hello.*", order=1)]
pub fn path_filter(mut req: HttpRequest) -> Chain {
    let attr = req.attributes_mut();
    attr.insert("kk".to_string(), "value".to_string());
    // let _method = req.method_mut();
    if req.headers().contains_key("token") {
        Chain::Continune(req)
    } else {
        let mut return_err = ReturnError::new(
            401, 
            "not authorized".to_string(),
        );
        return_err.headers.append("filter", HeaderValue::from_str("rejected").unwrap());
        Chain::Reject(return_err)
    }
    
}

#[tokio::main]
async fn main() {
    let server = Server::builder(([127, 0, 0, 1], 3000).into())
        .build()
        .await
        .unwrap();
    server.start().await.unwrap();
}