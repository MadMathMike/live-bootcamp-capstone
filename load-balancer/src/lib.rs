mod forwarding;
mod load_balancing;
mod monitoring;

use std::net::SocketAddr;
use std::ops::RangeInclusive;
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

pub struct LoadBalancerConfig {
    pub monitor_interval_secs: u16,
    pub ping_timeout_millis: u16,
    pub concurrent_connections_cutover: usize,
    pub listen_on_port: u16,
    pub host_port_range: RangeInclusive<u16>,
}

pub async fn run_load_balancer(
    config: LoadBalancerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = Arc::new(RwLock::new(Pool::new(
        config.concurrent_connections_cutover,
    )));

    let monitor_config = MonitorConfig {
        tick_interval_seconds: config.monitor_interval_secs,
        ping_timeout_millis: config.ping_timeout_millis,
        host_addresses: config
            .host_port_range
            .map(|port| SocketAddr::from(([127, 0, 0, 1], port)))
            .collect(),
    };

    tokio::task::spawn(monitor_pool(monitor_config, pool.clone()));

    let addr = SocketAddr::from(([127, 0, 0, 1], config.listen_on_port));
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
