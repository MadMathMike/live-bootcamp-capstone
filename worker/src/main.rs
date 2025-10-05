use worker::*;

use std::env;

use anyhow::{Result, anyhow};
use env_logger::{Builder, Target};

#[tokio::main]
async fn main() -> Result<()> {
    let mut builder = Builder::from_default_env();
    builder.target(Target::Stdout);
    builder.init();

    let max_work_milliseconds = env::var("MAX_WORK_MILLISECONDS")?.parse::<u64>()?;
    let include_ping = env::var("INCLUDE_PING")?.parse::<bool>()?;
    let port = env::var("WORKER_PORT")?.parse::<u16>()?;

    if !(3001..=3100).contains(&port) {
        return Err(anyhow!("WORKER_PORT must be in range [3001,3100]"));
    }

    let config = WorkerConfig {
        port,
        max_work_milliseconds,
        include_ping,
    };

    run_worker(config).await
}
