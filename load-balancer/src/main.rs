mod forwarding;
mod load_balancing;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{StatusCode, Uri};
use hyper_util::client;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::{interval, timeout};

use crate::forwarding::forward;
use crate::load_balancing::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addresses = vec![
        SocketAddr::from(([127, 0, 0, 1], 3001)),
        SocketAddr::from(([127, 0, 0, 1], 3002)),
        SocketAddr::from(([127, 0, 0, 1], 3003)),
    ];

    let pool = Arc::new(RwLock::new(Pool::default()));

    tokio::task::spawn(monitor_pool(addresses, pool.clone()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    let client = client::legacy::Builder::new(TokioExecutor::new()).build_http();

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

async fn monitor_pool(addresses: Vec<SocketAddr>, pool: Arc<RwLock<Pool>>) {
    // TODO: make this interval configurable
    let mut interval = interval(Duration::from_secs(10));
    let client: Client<HttpConnector, Full<Bytes>> =
        client::legacy::Builder::new(TokioExecutor::new()).build_http();

    loop {
        interval.tick().await;

        let mut good_hosts = vec![];
        let mut bad_hosts = vec![];

        // TODO: Ping all the hosts concurrently instead of serially
        for address in addresses.iter() {
            let uri = Uri::try_from(format!("http://{address}/ping")).unwrap();

            // TODO: make this timeout configurable
            let ping_response = timeout(Duration::from_millis(50), client.get(uri)).await;
            match ping_response {
                Ok(Ok(response)) if response.status() == StatusCode::OK => good_hosts.push(address),
                _ => bad_hosts.push(address),
            };
        }

        for bad_host in bad_hosts.iter() {
            println!("Ping failed on {bad_host}");
        }

        let mut mut_pool = pool.write().await;
        mut_pool.add_hosts(good_hosts);
        mut_pool.remove_hosts(bad_hosts);

        let algorithm = mut_pool.determine_algorithm();

        println!("{:?}", algorithm);
    }
}
