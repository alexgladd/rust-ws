use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;
use axum::{
    extract::{ws::{Message, WebSocket}, ConnectInfo, State, WebSocketUpgrade},
    http::{header::USER_AGENT, HeaderMap},
    response::IntoResponse,
    routing::get,
    Router,
};
use tokio::{net::TcpListener, sync::mpsc::Sender};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::db::Command;

#[derive(Debug)]
struct WsState {
    db_tx: Sender<Command>,
    token: CancellationToken,
}

pub(crate) async fn run_ws(db_tx: Sender<Command>, token: CancellationToken) -> Result<()> {
    let server_token = token.clone();

    // shared state for all handlers
    let state = Arc::new(WsState { db_tx, token });

    let app = Router::new()
        .route("/", get(root_handler))
        .route("/ws", get(ws_handler))
        .fallback(fallback_handler)
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;

    info!(target: "WS", "Bound on {}", listener.local_addr()?);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(server_token.cancelled_owned())
    .await?;

    info!(target: "WS", "Shutting down server");

    Ok(())
}

async fn fallback_handler() -> impl IntoResponse {
    "Not found"
}

async fn root_handler() -> impl IntoResponse {
    "Start a websocket connection! GET /ws"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<Arc<WsState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = match headers.get(USER_AGENT) {
        Some(h) => h.to_str().unwrap_or("Invalid").to_string(),
        None => "Unknown".to_string(),
    };

    info!(target: "WS", "Incoming connection from {} at {}", user_agent, addr);

    let db_tx = state.db_tx.clone();
    let token = state.token.clone();

    ws.on_failed_upgrade(|e| {
        error!(target: "WS", cause = %e, "Websocket upgrade failed!");
    })
    .on_upgrade(move |socket| handle_connection(socket, db_tx, token, addr))
}

async fn handle_connection(
    mut socket: WebSocket,
    mut _db_tx: Sender<Command>,
    token: CancellationToken,
    who: SocketAddr,
) {
    let mut stop = false;

    while !stop {
        tokio::select! {
            _ = token.cancelled() => {
                info!(target: "WS-C", "Cancellation requested; beginning shutdown for {}", who);
                stop = true;
            }
            msg = socket.recv() => {
                if let Some(msg) = msg {
                    if let Ok(msg) = msg {
                        match msg {
                            Message::Text(t) => {
                                info!(target: "WS-C", "Got text message from {}: {}", who, t);
                            }
                            Message::Close(_) => {
                                info!(target: "WS-C", "Client {} closed the socket", who);
                                stop = true;
                            }
                            _ => {
                                warn!(target: "WS-C", "Client {} sent an unsupported message type", who);
                            }
                        }
                    } else {
                        error!(target: "WS-C", "Failed to read message from {}; closing connection", who);
                        stop = true;
                    }
                } else {
                    info!(target: "WS-C", "Socket has been closed for {}", who);
                    stop = true;
                }
            }
        }
    }

    // make sure we close the socket
    let _ = socket.close().await;
}

// async fn process_msg()
