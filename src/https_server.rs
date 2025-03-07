use tokio::net::TcpListener;
use tokio_rustls::{TlsAcceptor, rustls::{ServerConfig, Certificate, PrivateKey}};
use std::sync::Arc;
use hyper::server::conn::Http;
use hyper::{Body, Request, Response};
use hyper::service::service_fn;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::fs;
use crate::Config;
use crate::router::handle_request;
use rcgen::generate_simple_self_signed;

fn load_certs(path: &str) -> Vec<Certificate> {
    let certfile = File::open(path).unwrap();
    let mut reader = BufReader::new(certfile);
    rustls_pemfile::certs(&mut reader).unwrap().into_iter().map(Certificate).collect()
}

fn load_keys(path: &str) -> Vec<PrivateKey> {
    let keyfile = File::open(path).unwrap();
    let mut reader = BufReader::new(keyfile);
    rustls_pemfile::pkcs8_private_keys(&mut reader).unwrap().into_iter().map(PrivateKey).collect()
}

fn generate_self_signed(cert_path: &str, key_path: &str) {
    use std::io::Write;
    let subject_alt_names = vec!["localhost".to_string()];
    let cert = generate_simple_self_signed(subject_alt_names).unwrap();
    let cert_pem = cert.serialize_pem().unwrap();
    let key_pem = cert.serialize_private_key_pem();
    fs::write(cert_path, cert_pem).unwrap();
    fs::write(key_path, key_pem).unwrap();
}

async fn handle(req: Request<Body>, config: Config) -> Result<Response<Body>, hyper::Error> {
    handle_request(req, config).await
}

pub async fn run_https(addr: &str, cert_path: &str, key_path: &str, config: Config) {
    if !Path::new(cert_path).exists() || !Path::new(key_path).exists() {
        generate_self_signed(cert_path, key_path);
    }
    let certs = load_certs(cert_path);
    let mut keys = load_keys(key_path);
    let server_config = ServerConfig::builder().with_safe_defaults().with_no_client_auth().with_single_cert(certs, keys.remove(0)).unwrap();
    let mut server_config = Arc::new(server_config);
    Arc::get_mut(&mut server_config).unwrap().alpn_protocols.push(b"h2".to_vec());
    Arc::get_mut(&mut server_config).unwrap().alpn_protocols.push(b"http/1.1".to_vec());
    let acceptor = TlsAcceptor::from(server_config);
    let listener = TcpListener::bind(addr).await.unwrap();
    loop {
        let (stream, _) = listener.accept().await.unwrap();
        let acceptor = acceptor.clone();
        let config = config.clone();
        tokio::spawn(async move {
            let tls_stream = acceptor.accept(stream).await.unwrap();
            let service = service_fn(move |req| {
                let config = config.clone();
                handle(req, config)
            });
            if let Err(e) = Http::new().serve_connection(tls_stream, service).await {
                eprintln!("HTTPS connection error: {}", e);
            }
        });
    }
}
// __proto__