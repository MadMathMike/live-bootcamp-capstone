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
use tokio::time::sleep;

use crate::forwarding::forward;
use crate::load_balancing::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = Arc::new(RwLock::new(Pool::new(vec![
        "localhost:3002".to_owned(),
        "localhost:3001".to_owned(),
        "localhost:3003".to_owned(),
    ])));

    tokio::task::spawn(monitor_pool(pool.clone()));

    let config_addr = SocketAddr::from(([127, 0, 0, 1], 2999));
    let config_listener = TcpListener::bind(config_addr).await?;
    tokio::task::spawn(listen_for_algorithm_change_requests(
        pool.clone(),
        config_listener,
    ));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    let client = client::legacy::Builder::new(TokioExecutor::new()).build(HttpConnector::new());

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let client = client.clone();
        let host = pool.write().await.next().unwrap();

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
}

async fn listen_for_algorithm_change_requests(pool: Arc<RwLock<Pool>>, listener: TcpListener) {
    loop {
        let (_, _) = listener.accept().await.unwrap();
        let mut mut_pool = pool.write().await;
        mut_pool.cycle_algorithm();
        for host in mut_pool.hosts.iter() {
            let response_times = host.response_times.read().await;
            let avg = if response_times.len() > 0 {
                Some(response_times.iter().sum::<u128>() / response_times.len() as u128)
            } else {
                None
            };
            println!("{} response time average {avg:?}", host.connection);
        }
        println!("{:?}", mut_pool.algorithm);
    }
}

// TODO: This is also where I will put health checks to pull hosts out of the pool when needed
async fn monitor_pool(pool: Arc<RwLock<Pool>>) {
    // This is a low number to make testing easier
    const CUTOFF: usize = 10;

    loop {
        let mut mut_pool = pool.write().await;
        mut_pool.algorithm = if mut_pool
            .hosts
            .iter()
            .any(|host| Host::count_connections(&host) > CUTOFF)
        {
            LoadBalancingAlgorithm::LeastConnections
        } else {
            LoadBalancingAlgorithm::RoundRobin
        };

        println!("{:?}", mut_pool.algorithm);

        sleep(Duration::from_secs(10)).await;
    }
}
