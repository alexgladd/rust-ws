use std::collections::HashMap;

use anyhow::Result;
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;
use tracing::info;

#[derive(Debug)]
pub(crate) enum Command {
    Get { key: String },
    Set { key: String, val: String },
}

#[derive(Debug)]
pub(crate) struct Value {
    pub key: String,
    pub val: String,
}

pub(crate) type Db = HashMap<String, String>;

pub(crate) async fn run_db(mut rx: Receiver<Command>, token: CancellationToken) -> Result<()> {
    let mut db = Db::new();

    let mut stop = false;

    while !stop {
        tokio::select! {
            _ = token.cancelled() => {
                info!("Cancellation requested; beginning shutdown");
                stop = true;
            }
            m = rx.recv() => {
                match m {
                    Some(c) => todo!(),
                    None => stop = true,
                }
            }
        }
    }

    Ok(())
}

async fn handle_command(c: Command, db: &mut Db) -> Result<Value> {
    match c {
        Command::Get { key } => Ok(Value {
            key: "foo".into(),
            val: "bar".into(),
        }),
        Command::Set { key, val } => Ok(Value {
            key: "foo".into(),
            val: "bar".into(),
        }),
    }
}
