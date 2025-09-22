mod forwarding;
mod load_balancing;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::forwarding::forward;
use crate::load_balancing::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = Arc::new(RwLock::new(Pool::new(vec![
        SocketAddr::from(([127, 0, 0, 1], 3001)),
        SocketAddr::from(([127, 0, 0, 1], 3002)),
        SocketAddr::from(([127, 0, 0, 1], 3003)),
    ])));

    // TODO: when health checks are implemented, update the monitor_pool
    // function to ping all the potential hosts at least once so we don't
    // start serving traffic to unresponsive hosts.
    tokio::task::spawn(monitor_pool(pool.clone()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    let client = client::legacy::Builder::new(TokioExecutor::new()).build(HttpConnector::new());

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        match pool.write().await.next_host() {
            Some(host) => {
                let client = client.clone();

                tokio::task::spawn(async move {
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
            None => todo!("Return 503 Service Unavailable response"),
        }
    }
}

async fn monitor_pool(pool: Arc<RwLock<Pool>>) {
    let mut interval = interval(Duration::from_secs(10));

    loop {
        interval.tick().await;

        // TODO: Check host health via /ping endpoint and remove from pool if no response

        let algorithm = pool.write().await.determine_algorithm();

        println!("{:?}", algorithm);
    }
}
