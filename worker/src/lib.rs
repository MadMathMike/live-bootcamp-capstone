use std::time::Duration;

use anyhow::Result;
use axum::{Router, extract::State, http::Response, routing::get};
use log::info;
use rand::{Rng, distr::Uniform};

pub struct WorkerConfig {
    pub port: u16,
    pub max_work_milliseconds: u64,
    pub include_ping: bool,
}

pub async fn run_worker(config: WorkerConfig) -> Result<()> {
    let range = Uniform::new(0, config.max_work_milliseconds).unwrap();

    let state = AppState {
        work_duration_range: range,
    };

    let mut app = Router::new().route("/work", get(work));

    // By excluding the ping, we make the worker look "unhealthy" to the load balancer because
    // the worker will return a 404 NOT FOUND response instead of the expected 200 OK response.
    if config.include_ping {
        // effectively our health check endpoint
        app = app.route("/ping", get(|| async {}));
    }

    let app = app.with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    info!("Listening on port {}", config.port);
    axum::serve(listener, app).await?;

    Ok(())
}

#[axum::debug_handler]
async fn work(State(state): State<AppState>) -> Response<String> {
    let work_duration = rand::rng().sample(state.work_duration_range);
    info!("Working for {work_duration}");
    tokio::time::sleep(Duration::from_millis(work_duration)).await;
    Response::new("Done!".to_owned())
}

#[derive(Clone)]
struct AppState {
    work_duration_range: Uniform<u64>,
}
