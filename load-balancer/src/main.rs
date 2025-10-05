use load_balancer::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LoadBalancerConfig {
        monitor_interval_secs: 10,
        ping_timeout_millis: 50,
        // This is a low number to make manual testing easier
        concurrent_connections_cutover: 10,
        listen_on_port: 3000,
        host_port_range: 3001..=3002,
    };

    run_load_balancer(config).await
}
