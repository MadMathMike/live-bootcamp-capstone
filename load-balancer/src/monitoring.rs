use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{StatusCode, Uri};
use hyper_util::client;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio::time::{interval, timeout};

use crate::pool::*;

pub struct MonitorConfig {
    pub tick_interval_seconds: u16,
    pub ping_timeout_millis: u16,
    pub host_addresses: Vec<SocketAddr>,
}

pub async fn monitor_pool(config: MonitorConfig, pool: Arc<RwLock<Pool>>) {
    let mut interval = interval(Duration::from_secs(config.tick_interval_seconds as u64));
    let client: Client<HttpConnector, Full<Bytes>> =
        client::legacy::Builder::new(TokioExecutor::new()).build_http();

    loop {
        interval.tick().await;

        let mut join_set = JoinSet::new();

        for address in config.host_addresses.iter().cloned() {
            let uri = Uri::try_from(format!("http://{address}/ping")).unwrap();
            let client = client.clone();

            join_set.spawn(async move {
                let ping_response = timeout(
                    Duration::from_millis(config.ping_timeout_millis as u64),
                    client.get(uri),
                )
                .await;
                match ping_response {
                    Ok(Ok(response)) if response.status() == StatusCode::OK => Ok(address),
                    _ => {
                        println!("Ping failed on {address}");
                        Err(address)
                    }
                }
            });
        }

        let ping_results = join_set.join_all().await;

        let (good_hosts, bad_hosts): (Vec<_>, Vec<_>) =
            ping_results.into_iter().partition(|result| result.is_ok());
        let good_hosts = good_hosts.into_iter().flatten().collect();
        let bad_hosts = bad_hosts.into_iter().flatten().collect();

        let mut mut_pool = pool.write().await;
        mut_pool.add_hosts(good_hosts);
        mut_pool.remove_hosts(bad_hosts);

        let algorithm = mut_pool.determine_algorithm();

        println!("{:?}", algorithm);
    }
}
