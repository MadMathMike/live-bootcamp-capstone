use std::time::Duration;

use load_balancer::*;
use worker::*;

// TODO: make the worker and load balancer bind to and return random, unused
// ports so we can run tests in parallel

#[tokio::test]
async fn load_balancer_forwards_requests() {
    // start worker
    let worker_config = WorkerConfig {
        port: 3001,
        include_ping: true,
        max_work_milliseconds: 10,
    };

    tokio::spawn(async move {
        _ = run_worker(worker_config).await;
    });

    // start load balancer
    let config = LoadBalancerConfig {
        monitor_interval_secs: 10,
        ping_timeout_millis: 50,
        concurrent_connections_cutover: 10,
        listen_on_port: 3000,
        host_port_range: 3001..=3001,
    };

    tokio::spawn(async move {
        _ = run_load_balancer(config).await;
    });

    // TODO: somehow ensure that the worker and load balancer are listening without waiting a fixed duration.
    // Perhaps they should both return a struct and should only return once their startup tasks (e.g., port binding)
    // are already complete.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // make request to load_balancer
    let response = reqwest::Client::new()
        .get("http://localhost:3000/work")
        .send()
        .await
        .unwrap();

    // assert!(response.is_ok());
    assert_eq!(reqwest::StatusCode::OK, response.status());
}

// The ports in this test differ from the other test to prevent collision
#[tokio::test]
async fn load_balancer_stops_routing_to_failed_hosts() {
    let url = format!("http://localhost:3010/work");

    tokio::spawn(async move {
        _ = run_worker(WorkerConfig {
            port: 3011,
            include_ping: true,
            max_work_milliseconds: 10,
        })
        .await;
    });

    let worker_to_stop = tokio::spawn(async move {
        _ = run_worker(WorkerConfig {
            port: 3012,
            include_ping: true,
            max_work_milliseconds: 10,
        })
        .await;
    });

    tokio::spawn(async move {
        _ = run_load_balancer(LoadBalancerConfig {
            monitor_interval_secs: 1,
            ping_timeout_millis: 50,
            concurrent_connections_cutover: 10,
            listen_on_port: 3010,
            host_port_range: 3011..=3012,
        })
        .await;
    });

    // sleep long enough for a health check to run
    tokio::time::sleep(Duration::from_secs(2)).await;

    let request_statuses = send_requests(&url, 6).await;

    assert_all_ok(&request_statuses);

    worker_to_stop.abort();

    // sleep long enough for a health check to run
    tokio::time::sleep(Duration::from_secs(2)).await;

    let request_statuses = send_requests(&url, 6).await;

    assert_all_ok(&request_statuses);
}

async fn send_requests(url: &str, count: usize) -> Vec<reqwest::StatusCode> {
    // Configure the client to keep no idle connections so that a new connection
    // is created for each request. This ensure the load balancer actually picks
    // the next host from its pool.
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .unwrap();

    let mut statuses = Vec::with_capacity(count);

    for _ in 0..count {
        let response = client.get(url).send().await.unwrap();
        statuses.push(response.status());
    }

    statuses
}

fn assert_all_ok(statuses: &[reqwest::StatusCode]) {
    assert!(
        statuses.iter().all(|status| status.is_success()),
        "expected every request to succeed, got: {statuses:?}"
    );
}
