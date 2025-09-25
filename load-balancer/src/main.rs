mod forwarding;
mod load_balancing;
mod monitoring;

use std::net::SocketAddr;
use std::sync::Arc;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::forwarding::forward;
use crate::load_balancing::*;
use crate::monitoring::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = Arc::new(RwLock::new(Pool::default()));

    let monitor_config = MonitorConfig {
        tick_interval_seconds: 10,
        ping_timeout_millis: 50,
        host_addresses: vec![
            SocketAddr::from(([127, 0, 0, 1], 3001)),
            SocketAddr::from(([127, 0, 0, 1], 3002)),
            SocketAddr::from(([127, 0, 0, 1], 3003)),
        ],
    };

    tokio::task::spawn(monitor_pool(monitor_config, pool.clone()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    let client = client::legacy::Builder::new(TokioExecutor::new()).build_http();

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let host = pool.write().await.next_host();
        let client = client.clone();

        tokio::task::spawn(async {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| forward(req, host.clone(), client.clone())),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
