use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use crate::Config;
use crate::router::handle_request;

async fn handle(req: Request<Body>, config: Config) -> Result<Response<Body>, hyper::Error> {
    handle_request(req, config).await
}

pub async fn run_http(addr: &str, config: Config) {
    let make_svc = make_service_fn(move |_conn| {
        let config = config.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                let config = config.clone();
                handle(req, config)
            }))
        }
    });
    let server = Server::bind(&addr.parse().unwrap()).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("HTTP server error: {}", e);
    }
}
// __proto__