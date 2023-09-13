use std::{pin::Pin, future::Future, net::SocketAddr};

use boring::ssl::{SslAcceptor, SslMethod};
use bytes::Bytes;

use http_body_util::Full;

use tokio::{net::TcpListener, runtime::Handle};
use regex::Regex;
use crate::{ROUTE_TABLE, support::tokiort::{TokioIo, TokioExecutor}, request::{HttpRequest, Body}, Predicate, response::Response};

#[derive(Clone, Copy)]
pub struct Service;

impl hyper::service::Service<hyper::Request<Body>> for Service {
    type Response = hyper::Response<Full<Bytes>>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: hyper::Request<Body>) -> Self::Future {

        println!("{}, {}", req.uri().path().to_string(), req.method().as_str().to_string());
        // reform the request
        let mut new_req = HttpRequest::new(req);


        // get the filter table
        let filter_table = crate::FILTER_TABLE.read().unwrap();
        for filter_info in filter_table.iter() {
            let path = new_req.path().to_string();
            let _method = new_req.method().as_str().to_string();
            let predicate = filter_info.predicate();
            let is_matched = match predicate {
                Predicate::PathPattern(pattern) => {
                    let reg_result = Regex::new(&pattern);
                    if reg_result.is_err() {
                        false
                    } else {
                        let reg = reg_result.unwrap();
                        reg.is_match(&path)
                    }
                }

            };

            if !is_matched {
                continue;
            }

            let handler = filter_info.handler();
            // let mut new_req = arc_new_req.clone();
            let chain = handler(new_req);

            let chain = tokio::task::block_in_place(move|| {
                Handle::current().block_on(chain)
            });
            // let rt = Runtime::new().unwrap();
            // let chain = rt.block_on(chain);
            match chain {
                crate::Chain::Continune(req) => {
                    new_req = req;
                }
                crate::Chain::Reject(err) => {
                    let mut builder = Response::builder();
                    for (k, v) in err.headers.iter() {
                        builder.headers_mut().unwrap().insert(k, v.clone());
                    }
                    let res = builder
                        .status(err.response_code)
                        .body(Full::new(Bytes::from(err.response_body.to_string()))).unwrap();
                    return Box::pin(async{Ok(res)});
                }
            }
        }

        let path = new_req.path().to_string();
        let method = new_req.method().as_str().to_string();

        // check if the request path and method is exist in route table
        let router_table = ROUTE_TABLE.read().unwrap();
        let route_info = router_table.iter().find(|route| {
            route.method() == method && route.path().pattern_match(&path).is_some()
        }).map(|route| {
            route
        });
        // if the service exists, call the service function
        let res = if let Some(r) = route_info.clone() {
            let handler = r.handler();
            let path_params = r.path().pattern_match(&path);
            if path_params.is_some() {
                new_req.set_path_params(path_params.unwrap());
            }

            handler(new_req)
        } else {
            // if the service not exists, return 404
            Box::pin(async{Ok(hyper::Response::builder().status(hyper::StatusCode::NOT_FOUND).body(Full::new(Bytes::from("not found!"))).unwrap())})
            
        };

        res

    }
}

pub enum SslFiletype {
    PEM,
    ASN1,
}

impl SslFiletype {
    fn to_boring(&self) -> boring::ssl::SslFiletype {
        match self {
            SslFiletype::PEM => boring::ssl::SslFiletype::PEM,
            SslFiletype::ASN1 => boring::ssl::SslFiletype::ASN1,
        }
    }
}

pub struct ServerBuilder {
    addr: SocketAddr,
    protocol: Protocol,
    ssl_acceptor: Option<SslAcceptor>,
}

impl ServerBuilder {
    fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            protocol: Protocol::HTTP1,
            ssl_acceptor: None,
        }
    }

    pub fn ssl(&mut self, private_key_file: &str, server_cert_file: &str, ssl_file_type: SslFiletype) -> Result<&mut Self, Box<dyn std::error::Error>> {
        let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
        acceptor.set_private_key_file(private_key_file, ssl_file_type.to_boring())?;
        acceptor.set_certificate_chain_file(server_cert_file)?;
        self.ssl_acceptor = Some(acceptor.build());
        Ok(self)
    }

    pub fn h1(&mut self) -> &mut Self {
        self.protocol = Protocol::HTTP1;
        self
    }

    pub fn h2(&mut self) -> &mut Self {
        self.protocol = Protocol::HTTP2;
        self
    }

    pub async fn build(&self) -> Result<Server, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(self.addr).await?;
        Ok(Server { 
            listener,  
            service: Service, 
            protocol: self.protocol,
            ssl_acceptor: self.ssl_acceptor.clone(),
        })
    }

}

pub struct Server {
    listener: TcpListener,
    service: Service,
    protocol: Protocol, 
    ssl_acceptor: Option<SslAcceptor>,
}

impl Server {
    pub fn builder(addr: SocketAddr) -> ServerBuilder {
        ServerBuilder::new(addr)
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            if let Some(acceptor) = &self.ssl_acceptor {
                // let stream = tokio_boring::accept(&acceptor, stream).await;
                if let Ok(stream) = tokio_boring::accept(&acceptor, stream).await {
                    let io = TokioIo::new(stream);
                    tokio::task::spawn(async move {
                        match self.protocol {
                            Protocol::HTTP1 => {
                                if let Err(err) = hyper::server::conn::http1::Builder::new()
                                .serve_connection(
                                    io,
                                    self.service,
                                ).await
                                {
                                    println!("Failed to serve connection: {}", err.message());
                                }
                            },
                            Protocol::HTTP2 => {
                                if let Err(err) = hyper::server::conn::http2::Builder::new(TokioExecutor)
                                .serve_connection(
                                    io,
                                    self.service,
                                ).await
                                {
                                    println!("Failed to serve h2 connection: {}", err.to_string());
                                }
                            },
                            
                        };
                        
                    });
                } else {
                    println!("Failed to accept ssl connection");
                }
            } else {
                let io = TokioIo::new(stream);
                tokio::task::spawn(async move {
                    
                    match self.protocol {
                        Protocol::HTTP1 => {
                            if let Err(err) = hyper::server::conn::http1::Builder::new()
                            .serve_connection(
                                io,
                                self.service,
                            ).await
                            {
                                println!("Failed to serve connection: {}", err.message());
                            }
                        },
                        Protocol::HTTP2 => {
                            if let Err(err) = hyper::server::conn::http2::Builder::new(TokioExecutor)
                            .serve_connection(
                                io,
                                self.service,
                            ).await
                            {
                                println!("Failed to serve h2 connection: {}", err.message());
                            }
                        },
                        
                    };
                    
                });
            };
            
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum  Protocol {
    HTTP1,
    HTTP2,
}