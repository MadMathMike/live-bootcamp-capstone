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
