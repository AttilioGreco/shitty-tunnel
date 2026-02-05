use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use http_body_util::BodyExt;

use st_domain::model::request::ProxiedRequest;

use crate::app::AppState;

static REQUEST_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> u64 {
    REQUEST_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/{*path}", any(handle_request))
        .route("/", any(handle_request))
        .with_state(state)
}

async fn handle_request(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> Response {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let domain = host.split(':').next().unwrap_or(host);

    let tunnel = {
        let tunnels = state.tunnels.read().await;
        match tunnels.get(domain) {
            Some(t) => t.request_tx.clone(),
            None => {
                tracing::warn!("no tunnel for domain: {domain}");
                return (StatusCode::BAD_GATEWAY, "no tunnel connected for this domain")
                    .into_response();
            }
        }
    };

    let (parts, body) = req.into_parts();

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            tracing::error!("failed to read request body: {e}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let request_id = next_request_id();
    let proxied = ProxiedRequest {
        request_id,
        method: parts.method.to_string(),
        uri: parts.uri.to_string(),
        headers: parts
            .headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect(),
        body: body_bytes.to_vec(),
    };

    let (response_tx, response_rx) = tokio::sync::oneshot::channel();
    let pending = crate::app::PendingRequest {
        request: proxied,
        response_tx,
    };

    if tunnel.send(pending).await.is_err() {
        return (StatusCode::BAD_GATEWAY, "tunnel disconnected").into_response();
    }

    match tokio::time::timeout(Duration::from_secs(30), response_rx).await {
        Ok(Ok(resp)) => {
            let mut builder = Response::builder().status(resp.status);
            for (key, value) in &resp.headers {
                builder = builder.header(key.as_str(), value.as_str());
            }
            builder
                .body(Body::from(resp.body))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Ok(Err(_)) => (StatusCode::BAD_GATEWAY, "tunnel dropped response").into_response(),
        Err(_) => (StatusCode::GATEWAY_TIMEOUT, "request timed out").into_response(),
    }
}
