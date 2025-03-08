use hyper::{Body, Request, Response, StatusCode};
use hyper::header::{HeaderValue, CONTENT_TYPE, STRICT_TRANSPORT_SECURITY};
use hyper::Client;
use hyper::Uri;
use mime_guess::from_path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use std::str::FromStr;
use std::path::Path;
use crate::Config;
use crate::middleware::metrics;

fn add_security_headers(mut resp: Response<Body>, level: u8, is_https: bool) -> Response<Body> {
    let headers = resp.headers_mut();
    if level >= 1 {
        headers.insert("X-XSS-Protection", HeaderValue::from_static("1; mode=block"));
        headers.insert("Anti-Xss", HeaderValue::from_static("enabled"));
    }
    if level >= 2 {
        headers.insert("X-Content-Type-Options", HeaderValue::from_static("nosniff"));
    }
    if level >= 3 {
        headers.insert("X-Frame-Options", HeaderValue::from_static("DENY"));
        headers.insert("Content-Security-Policy", HeaderValue::from_static("default-src 'self'; script-src 'self'"));
        headers.insert("Referrer-Policy", HeaderValue::from_static("no-referrer"));
        headers.insert("Permissions-Policy", HeaderValue::from_static("geolocation=(), microphone=(), camera=()"));
    }
    headers.insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    if is_https {
        headers.insert("Strict-Transport-Security", HeaderValue::from_static("max-age=31536000; includeSubDomains"));
    }
    resp
}

async fn handle_reverse_proxy(req: Request<Body>, target: &str, security_level: u8, is_https: bool) -> Result<Response<Body>, hyper::Error> {
    let path_and_query = req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    let new_uri_str = format!("{}{}", target.trim_end_matches('/'), path_and_query);
    let new_uri = Uri::from_str(&new_uri_str).unwrap();
    let mut proxied_req = Request::builder().method(req.method()).uri(new_uri);
    for (key, value) in req.headers().iter() { proxied_req = proxied_req.header(key, value); }
    let body = req.into_body();
    let proxied_req = proxied_req.body(body).unwrap();
    let client = Client::new();
    let resp = client.request(proxied_req).await?;
    Ok(add_security_headers(resp, security_level, is_https))
}

async fn handle_static_file(file_path: &str, config: &Config, security_level: u8, is_https: bool) -> Result<Response<Body>, hyper::Error> {
    let full_path = if Path::new(file_path).is_absolute() { file_path.to_string() } else { format!("{}/{}", config.static_dir, file_path) };
    match File::open(&full_path).await {
        Ok(mut file) => {
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).await.unwrap();
            let mime_type = from_path(&full_path).first_or_octet_stream();
            let resp = Response::builder().header(CONTENT_TYPE, mime_type.as_ref()).body(Body::from(contents)).unwrap();
            Ok(add_security_headers(resp, security_level, is_https))
        },
        Err(_) => {
            let resp = Response::builder().status(StatusCode::NOT_FOUND).body(Body::from("Not Found")).unwrap();
            Ok(add_security_headers(resp, security_level, is_https))
        }
    }
}

async fn handle_default_static(req: &Request<Body>, config: &Config, security_level: u8, is_https: bool) -> Result<Response<Body>, hyper::Error> {
    let path = req.uri().path();
    let safe_path = if path == "/" { "index.html".to_string() } else { path.trim_start_matches('/').to_string() };
    handle_static_file(&safe_path, config, security_level, is_https).await
}

pub async fn handle_request(req: Request<Body>, config: Config) -> Result<Response<Body>, hyper::Error> {
    let is_https = config.enable_https;
    if req.method() == hyper::Method::OPTIONS {
        return Ok(Response::builder().status(StatusCode::NO_CONTENT)
          .header("Access-Control-Allow-Origin", "*")
          .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
          .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
          .body(Body::empty()).unwrap());
    }
    if req.uri().path() == "/health" {
        return Ok(Response::builder().status(StatusCode::OK)
          .header(CONTENT_TYPE, "text/plain")
          .body(Body::from("OK")).unwrap());
    }
    if req.uri().path() == "/metrics" {
        let m = metrics();
        return Ok(Response::builder().status(StatusCode::OK)
          .header(CONTENT_TYPE, "text/plain")
          .body(Body::from(m)).unwrap());
    }
    for entry in config.static_routes.iter() {
        if req.uri().path() == entry.path {
            return handle_static_file(&entry.file, &config, config.security_level, is_https).await;
        }
    }
    for entry in config.reverse_proxy_routes.iter() {
        if req.uri().path().starts_with(&entry.path) {
            return handle_reverse_proxy(req, &entry.target, config.security_level, is_https).await;
        }
    }
    if config.proxy_enabled && req.uri().path().starts_with(&config.proxy_route) {
        return handle_reverse_proxy(req, &config.proxy_target, config.security_level, is_https).await;
    }
    if req.method() == hyper::Method::GET {
        return handle_default_static(&req, &config, config.security_level, is_https).await;
    }
    let resp = Response::builder().body(Body::from("Hello from Haxxserver")).unwrap();
    Ok(add_security_headers(resp, config.security_level, is_https))
}
