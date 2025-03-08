use std::fs;
use std::path::{Path, PathBuf};
use serde::Deserialize;

#[derive(Deserialize)]
struct ConfFile {
    server: ServerConfig,
    proxy: Option<ProxyConfig>,
    reverse_proxy: Option<ReverseProxyConfig>,
    static_routes: Option<StaticRoutesConfig>,
}

#[derive(Deserialize)]
struct ServerConfig {
    http_addr: Option<String>,
    https_addr: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
    static_dir: Option<String>,
    security_level: Option<u8>,
    enable_https: Option<bool>,
}

#[derive(Deserialize)]
struct ProxyConfig {
    enable: Option<bool>,
    route: Option<String>,
    target: Option<String>,
}

#[derive(Deserialize)]
struct ReverseProxyConfig {
    enable: Option<bool>,
    routes: Option<Vec<ReverseProxyEntry>>,
}

#[derive(Deserialize, Clone)]
pub struct ReverseProxyEntry {
    pub path: String,
    pub target: String,
}

#[derive(Deserialize)]
struct StaticRoutesConfig {
    routes: Option<Vec<StaticRouteEntry>>,
}

#[derive(Deserialize, Clone)]
pub struct StaticRouteEntry {
    pub path: String,
    pub file: String,
}

#[derive(Clone)]
pub struct Config {
    pub http_addr: String,
    pub https_addr: String,
    pub cert_path: String,
    pub key_path: String,
    pub static_dir: String,
    pub security_level: u8,
    pub enable_https: bool,
    pub proxy_enabled: bool,
    pub proxy_route: String,
    pub proxy_target: String,
    pub reverse_proxy_enabled: bool,
    pub reverse_proxy_routes: Vec<ReverseProxyEntry>,
    pub static_routes: Vec<StaticRouteEntry>,
}

impl Config {
    pub fn new(conf_file: &str) -> Self {
        if !Path::new(conf_file).exists() {
            let default = r#"
# Haxxserver configuration file
#
# [server]
# http_addr: HTTP bind address (e.g. "127.0.0.1:8080")
# https_addr: HTTPS bind address (e.g. "127.0.0.1:8443")
# cert_path: Path to TLS certificate (for HTTPS)
# key_path: Path to TLS private key (for HTTPS)
# static_dir: Directory for static files (HTML, CSS, etc.)
# security_level: Security level from 0 (none) to 3 (maximum)
# enable_https: true to enable HTTPS, false to disable it.
#
# [proxy]
# enable: Enable single reverse proxy endpoint.
# route: Route prefix for the proxy.
# target: Target URL for the proxy.
#
# [reverse_proxy]
# enable: Enable reverse proxy mappings.
# routes: Array of mappings with 'path' and 'target'.
#
# [static_routes]
# routes: Array of mappings with 'path' and 'file'. The file path is relative to static_dir if not absolute.
 
[server]
http_addr = "127.0.0.1:8080"
https_addr = "127.0.0.1:8443"
cert_path = "certs/cert.pem"
key_path = "certs/key.pem"
static_dir = "public"
security_level = 3
enable_https = true

[proxy]
enable = false
route = "/api/"
target = "http://localhost:3000"

[reverse_proxy]
enable = true
routes = [
  { path = "/api/", target = "http://localhost:3000" },
  { path = "/google/", target = "https://google.com" }
]

[static_routes]
routes = [
  { path = "/login", file = "cadastro/login.html" },
  { path = "/about", file = "info/about.html" }
]
"#;
            fs::write(conf_file, default).unwrap();
        }
        let content = fs::read_to_string(conf_file).unwrap();
        let file_config: ConfFile = toml::from_str(&content).unwrap();
        let server = file_config.server;
        let proxy = file_config.proxy.unwrap_or(ProxyConfig {
            enable: Some(false),
            route: Some(String::new()),
            target: Some(String::new()),
        });
        let reverse_proxy = file_config.reverse_proxy.unwrap_or(ReverseProxyConfig {
            enable: Some(false),
            routes: Some(Vec::new()),
        });
        let static_routes_config = file_config.static_routes.unwrap_or(StaticRoutesConfig {
            routes: Some(Vec::new()),
        });
        let http_addr = server.http_addr.unwrap_or_else(|| "127.0.0.1:8080".to_string());
        let https_addr = server.https_addr.unwrap_or_else(|| "127.0.0.1:8443".to_string());
        let cert_path = server.cert_path.unwrap_or_else(|| "certs/cert.pem".to_string());
        let key_path = server.key_path.unwrap_or_else(|| "certs/key.pem".to_string());
        let static_dir_raw = server.static_dir.unwrap_or_else(|| "public".to_string());
        let static_dir = {
            let p = PathBuf::from(static_dir_raw);
            if p.is_absolute() { p } else { std::env::current_dir().unwrap().join(p) }
        }
        .to_string_lossy()
        .to_string();
        let security_level = server.security_level.unwrap_or(0);
        let enable_https = server.enable_https.unwrap_or(true);
        let proxy_enabled = proxy.enable.unwrap_or(false);
        let proxy_route = proxy.route.unwrap_or_else(|| "/api/".to_string());
        let proxy_target = proxy.target.unwrap_or_else(|| "http://localhost:3000".to_string());
        let reverse_proxy_enabled = reverse_proxy.enable.unwrap_or(false);
        let reverse_proxy_routes = reverse_proxy.routes.unwrap_or(Vec::new());
        let static_routes = static_routes_config.routes.unwrap_or(Vec::new());
        Self {
            http_addr,
            https_addr,
            cert_path,
            key_path,
            static_dir,
            security_level,
            enable_https,
            proxy_enabled,
            proxy_route,
            proxy_target,
            reverse_proxy_enabled,
            reverse_proxy_routes,
            static_routes,
        }
    }
    pub fn setup(&self) {
        if let Some(parent) = Path::new(&self.cert_path).parent() {
            if !parent.exists() { fs::create_dir_all(parent).unwrap(); }
        }
        if !Path::new(&self.static_dir).exists() { fs::create_dir_all(&self.static_dir).unwrap(); }
    }
}
