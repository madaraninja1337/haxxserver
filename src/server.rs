use hyper::{Body, Request, Response, Server};
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use crate::Config;
use crate::router::handle_request;
use std::time::Instant;
use std::net::SocketAddr;
use crate::middleware::check_rate_limit;

async fn log_handle(req: Request<Body>, config: Config, remote: SocketAddr) -> Result<Response<Body>, hyper::Error> {
    if !check_rate_limit(remote) {
        return Ok(Response::builder().status(429).body(Body::from("Too Many Requests")).unwrap());
    }
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let resp = handle_request(req, config).await;
    let elapsed = start.elapsed();
    match &resp {
        Ok(response) => println!("{} {} {} {}ms [{}]", method, uri, response.status(), elapsed.as_millis(), remote),
        Err(_) => println!("{} {} ERROR {}ms [{}]", method, uri, elapsed.as_millis(), remote),
    }
    resp
}

pub async fn run_http(addr: &str, config: Config) {
    let make_svc = make_service_fn(move |conn: &AddrStream| {
        let remote = conn.remote_addr();
        let config = config.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                let config = config.clone();
                log_handle(req, config, remote)
            }))
        }
    });
    let server = Server::bind(&addr.parse().unwrap()).serve(make_svc);
    if let Err(e) = server.await { eprintln!("HTTP server error: {}", e); }
}
