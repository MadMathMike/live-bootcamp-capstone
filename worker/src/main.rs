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

    let state = AppState {
        work_duration_range: range,
    };

    let app = Router::new()
        .route("/work", get(work))
        // effectively our health check endpoint
        .route("/ping", get(|| async {}))
        .with_state(state);

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
