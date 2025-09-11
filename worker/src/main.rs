use std::time::Duration;

use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route(
            "/work",
            get(|| async {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }),
        )
        // effectively our health check endpoint
        .route("/ping", get(|| async {}));

    // TODO: probably get the port from an environment variable so when we spin
    // up multiple workers in docker, we can configure their port binding more easily
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
