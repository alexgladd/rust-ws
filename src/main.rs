mod db;

use anyhow::Result;
use db::run_db;
use std::env;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};
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
    let (tx, mut rx) = mpsc::channel(1024);

    let db_task = tokio::spawn(async move { run_db(rx, db_token).await });

    let t1 = tokio::spawn(async {
        run_for(5).await;
    });

    let t2 = tokio::spawn(async {
        run_for(10).await;
    });

    info!("Running; shut down with CTRL-C (SIGINT)");
    match tokio::signal::ctrl_c().await {
        Ok(_) => {}
        Err(e) => error!(%e, "Unable to listen for SIGINT; aborting"),
    }

    token.cancel();

    info!("Done!");

    Ok(())
}

async fn run_for(secs: u64) {
    debug!("Async sleeping for {} seconds", secs);
    tokio::time::sleep(Duration::from_secs(secs)).await;
    debug!("Done async sleeping");
}
