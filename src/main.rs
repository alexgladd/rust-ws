mod db;

use crate::db::{run_db, Command, CommandResult};
use anyhow::Result;
use std::env;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
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
    let (tx1, rx) = mpsc::channel(1024);
    let tx2 = tx1.clone();

    let db_task = tokio::spawn(async move { run_db(rx, db_token).await });

    let t1 = tokio::spawn(async move {
        let secs: u64 = 5;
        debug!(target: "T1", "Async sleeping for {} seconds", secs);
        tokio::time::sleep(Duration::from_secs(secs)).await;
        debug!(target: "T1", "Done async sleeping");

        let (txres, rxres) = oneshot::channel();

        match tx1
            .send(Command::Set {
                key: "foo".into(),
                val: "foobar1234".into(),
                resp: txres,
            })
            .await
        {
            Ok(_) => {}
            Err(_) => error!(target: "T1", "Failed to send command!"),
        };

        match rxres.await {
            Ok(res) => match res {
                CommandResult::Ok { key, val: _ } => info!(target: "T1", "Set value for {}", key),
                CommandResult::NotFound => {}
                CommandResult::Err { msg } => {
                    error!(target: "T1", "Error setting value: {}", msg)
                }
            },
            Err(e) => error!(target: "T1", cause = %e, "Failed to receive command result!"),
        };

        info!(target: "T1", "Done.");
    });

    let t2 = tokio::spawn(async move {
        let secs: u64 = 10;
        debug!(target: "T2", "Async sleeping for {} seconds", secs);
        tokio::time::sleep(Duration::from_secs(secs)).await;
        debug!(target: "T2", "Done async sleeping");

        let (txres, rxres) = oneshot::channel();

        match tx2
            .send(Command::Get {
                key: "foo".into(),
                resp: txres,
            })
            .await
        {
            Ok(_) => {}
            Err(_) => error!(target: "T2", "Failed to send command!"),
        };

        match rxres.await {
            Ok(res) => match res {
                CommandResult::Ok { key, val } => {
                    info!(target: "T2", "Got value for {}: {}", key, val)
                }
                CommandResult::NotFound => {}
                CommandResult::Err { msg } => {
                    error!(target: "T2", "Error setting value: {}", msg)
                }
            },
            Err(e) => error!(target: "T2", cause = %e, "Failed to receive command result!"),
        };

        info!(target: "T2", "Done.");
    });

    info!("Running; shut down with CTRL-C (SIGINT)");
    match tokio::signal::ctrl_c().await {
        Ok(_) => {}
        Err(e) => error!(cause = %e, "Unable to listen for SIGINT; aborting"),
    }

    token.cancel();

    info!("Done!");

    Ok(())
}
