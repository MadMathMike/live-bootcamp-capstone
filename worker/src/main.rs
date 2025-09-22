use std::{env, time::Duration};

use anyhow::{Result, anyhow};
use axum::{Router, extract::State, http::Response, routing::get};
use env_logger::{Builder, Target};
use log::info;
use rand::{Rng, distr::Uniform};

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

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

    let max_work_milliseconds = env::var("MAX_WORK_MILLISECONDS")?.parse::<u64>()?;
    let range = Uniform::new(0, max_work_milliseconds).unwrap();

    let include_ping = env::var("INCLUDE_PING")?.parse::<bool>()?;

    let state = AppState {
        work_duration_range: range,
    };

    let mut app = Router::new().route("/work", get(work));

    // By excluding the ping, we make the worker look "unhealthy" to the load balancer because
    // the worker will return a 404 NOT FOUND response instead of the expected 200 OK response.
    if include_ping {
        // effectively our health check endpoint
        app = app.route("/ping", get(|| async {}));
    }

    let app = app.with_state(state);

    let port_number = env::var("WORKER_PORT")?.parse::<u16>()?;

    match port_number {
        3001..=3100 => {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port_number}")).await?;
            info!("Listening on port {port_number}");
            axum::serve(listener, app).await?;

            Ok(())
        }
        _ => Err(anyhow!("WORKER_PORT must be in range [3001,3100]")),
    }
}
