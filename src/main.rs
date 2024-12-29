mod db;
mod ws;

use crate::db::run_db;
use crate::ws::run_ws;
use anyhow::Result;
use std::env;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

fn init_logging() -> Result<()> {
    // logging filter: default to info for everything or use filter(s) from env RUST_LOG
    let filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::try_new("info")?);
    // logging format: add extra info if the RUST_LOG env is set (assuming dev environment)
    let format = if env::var("RUST_LOG").is_ok() {
        fmt::layer()
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
    } else {
        fmt::layer()
    };

    // init logging subscriber
    tracing_subscriber::registry()
        .with(filter)
        .with(format)
        .init();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging()?;

    info!(
        "Rust Websocket Example Server v{}",
        env!("CARGO_PKG_VERSION")
    );

    let token = CancellationToken::new();

    info!("Setting up DB task...");
    let db_token = token.clone();
    let ws_token = token.clone();
    let (tx, rx) = mpsc::channel(1024);

    let db_task = tokio::spawn(async move { run_db(rx, db_token).await });

    let ws_task = tokio::spawn(async move { run_ws(tx, ws_token).await });

    info!("Running; shut down with CTRL-C (SIGINT)");
    match tokio::signal::ctrl_c().await {
        Ok(_) => {}
        Err(e) => error!(cause = %e, "Unable to listen for SIGINT; aborting"),
    }

    token.cancel();

    let _ = tokio::join!(db_task, ws_task);

    info!("Done!");

    Ok(())
}
