use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use tokio::sync::{mpsc::Receiver, oneshot::Sender};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub(crate) enum Command {
    Get {
        key: String,
        resp: Responder,
    },
    Set {
        key: String,
        val: String,
        resp: Responder,
    },
}

#[derive(Debug)]
pub(crate) enum CommandResult {
    Ok { key: String, val: String },
    NotFound,
    Err { msg: String },
}

#[derive(Debug)]
pub(crate) struct Value {
    pub key: String,
    pub val: String,
}

pub(crate) type Db = HashMap<String, String>;
pub(crate) type Responder = Sender<CommandResult>;

pub(crate) async fn run_db(mut rx: Receiver<Command>, token: CancellationToken) -> Result<()> {
    let mut db = Db::new();

    let mut stop = false;

    while !stop {
        tokio::select! {
            _ = token.cancelled() => {
                info!(target: "DB", "Cancellation requested; beginning shutdown");
                rx.close();
                stop = true;
            }
            m = rx.recv() => {
                match m {
                    Some(c) => handle_command(c, &mut db).await,
                    None => {
                        warn!(target: "DB", "Receive channel closed; beginning shutdown");
                        stop = true
                    },
                }
            }
        }
    }

    Ok(())
}

async fn handle_command(c: Command, db: &mut Db) {
    match c {
        Command::Get { key, resp } => {
            debug!(target: "DB", "GET request for {}", key);

            // simulate latency with a random sleep
            tokio::time::sleep(Duration::from_millis(fastrand::u64(10..=100))).await;

            // retrieve the requested value
            let result = match db.get(&key) {
                Some(val) => {
                    debug!(target: "DB", "Found value for key {}", key);
                    CommandResult::Ok {
                        key,
                        val: val.clone(),
                    }
                }
                None => {
                    debug!(target: "DB", "No value found for key {}", key);
                    CommandResult::NotFound
                }
            };

            // send a response via the responder
            match resp.send(result) {
                Ok(_) => debug!(target: "DB", "Sent GET response"),
                Err(_) => error!(target: "DB", "Failed to send GET response!"),
            };
        }
        Command::Set { key, val, resp } => {
            debug!(target: "DB", "SET request for {}", key);

            // simulate latency with a random sleep
            tokio::time::sleep(Duration::from_millis(fastrand::u64(50..=200))).await;

            // insert the given value
            match db.insert(key.clone(), val.clone()) {
                Some(old_val) => {
                    debug!(target: "DB", "Replaced existing value {} for key {}", old_val, key)
                }
                None => debug!(target: "DB", "Inserted new key {} with value {}", key, val),
            };

            let result = CommandResult::Ok { key, val };

            // send the response via the responder
            match resp.send(result) {
                Ok(_) => debug!(target: "DB", "Sent SET response"),
                Err(_) => error!(target: "DB", "Failed to send SET response!"),
            };
        }
    }
}
