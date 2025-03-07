mod config;
mod router;
mod server;
mod https_server;

use config::Config;
use clap::Parser;
use tokio::join;

#[derive(Parser)]
#[clap(author = "Haxxserver", version = "0.1.0", about = "A secure and feature rich webserver")]
struct Args {
    #[clap(short, long, default_value = "./haxxserver.conf")]
    config: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let cfg = Config::new(&args.config);
    cfg.setup();
    println!("HTTP listening on {}", cfg.http_addr);
    if cfg.enable_https {
        println!("HTTPS listening on {}", cfg.https_addr);
        let http = server::run_http(&cfg.http_addr, cfg.clone());
        let https = https_server::run_https(&cfg.https_addr, &cfg.cert_path, &cfg.key_path, cfg.clone());
        join!(http, https);
    } else {
        println!("HTTPS is disabled.");
        server::run_http(&cfg.http_addr, cfg.clone()).await;
    }
}
// __proto__