use std::{env, time::Duration};

use anyhow::{Result, anyhow};
use axum::{Router, http::Response, routing::get};
use env_logger::{Builder, Target};
use log::info;

#[axum::debug_handler]
async fn work() -> Response<String> {
    info!("Doing work!");
    tokio::time::sleep(Duration::from_millis(2000)).await;
    Response::new("Done!".to_owned())
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);

    builder.init();

    let app = Router::new()
        .route("/work", get(work))
        // effectively our health check endpoint
        .route("/ping", get(|| async {}));

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
