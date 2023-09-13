# Argos

Argos makes it easy to create a stand-alone web application backend server.

## Usage

### Simple Example

#### Step 1: Define your `API` function

You can define your api function with a macro-attribute:

```rust
#[route(GET, path = "/api/hello", formatter = "text")]
pub fn hello(req: HttpRequest) -> Result<String, ReturnError<String>> {
    let url_params = req.url_params();
    let name = url_params.get("name");
    match name {
        Some(name) => Ok(format!("hello {}!", name)),
        None => Err(
            ReturnError::new(
                400,
                "name is required".to_string(),
            )
        ),
    }
}
```
> this function define a http interface with `GET` method, and url path is `/api/hello`.

#### Step 2: Start a server

You can start your server like this:
```rust
#[tokio::main]
async fn main() {
    let server = Server::builder(([127, 0, 0, 1], 3000).into())
        .build()
        .await
        .unwrap();
    server.start().await.unwrap();
}
```

Your server will bind in `127.0.0.1:3000`, so you can send a http request to call this API:

```shell
curl http://127.0.0.1:3000/api/hello?name=liudao
```

### Filter

You can define a filter to filt the request:

```rust
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
```

> Filter now only support `path_pattern`, using `regexp`. The attribute `order` represents the priority of the filter, the smaller the number, the higher the priority.

See more examples in `core/examples`.

## License

Argos is provided under the MIT license. See [LICENSE](https://github.com/jimmyseraph/argos/blob/main/LICENSE).