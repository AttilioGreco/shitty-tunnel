use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::http::{header, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use rust_embed::Embed;

use super::buffer::{EventBuffer, WsMessage};

#[derive(Embed)]
#[folder = "../../frontend/dist"]
struct FrontendAssets;

pub fn router(buffer: Arc<EventBuffer>) -> Router {
    Router::new()
        .route("/api/ws", get(ws_handler))
        .route("/api/events", get(events_handler))
        .with_state(buffer)
        .fallback(static_handler)
}

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try the exact path first
    if !path.is_empty() {
        if let Some(file) = FrontendAssets::get(path) {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                file.data,
            )
                .into_response();
        }
    }

    // SPA fallback: serve index.html for all other routes
    match FrontendAssets::get("index.html") {
        Some(file) => Html(file.data).into_response(),
        None => (StatusCode::NOT_FOUND, "frontend not built").into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(buffer): State<Arc<EventBuffer>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, buffer))
}

async fn handle_ws(mut socket: WebSocket, buffer: Arc<EventBuffer>) {
    // Send initial snapshot
    let (events, epoch_ms) = buffer.snapshot().await;
    let snapshot = WsMessage::Snapshot { events, epoch_ms };
    if let Ok(json) = serde_json::to_string(&snapshot) {
        if socket.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    let mut rx = buffer.subscribe();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(ws_msg) => {
                        if let Ok(json) = serde_json::to_string(&ws_msg) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Client fell behind — send fresh snapshot
                        let (events, epoch_ms) = buffer.snapshot().await;
                        let snapshot = WsMessage::Snapshot { events, epoch_ms };
                        if let Ok(json) = serde_json::to_string(&snapshot) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<ClientCommand>(&text) {
                            match cmd {
                                ClientCommand::Clear => buffer.clear().await,
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

async fn events_handler(State(buffer): State<Arc<EventBuffer>>) -> impl IntoResponse {
    let (events, epoch_ms) = buffer.snapshot().await;
    let snapshot = WsMessage::Snapshot { events, epoch_ms };
    axum::Json(snapshot)
}

use tokio::sync::broadcast;

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
enum ClientCommand {
    #[serde(rename = "clear")]
    Clear,
}

pub async fn run(buffer: Arc<EventBuffer>, port: u16) -> anyhow::Result<()> {
    let app = router(buffer);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("dashboard listening on {addr}");
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("dashboard server error: {e}"))
}
