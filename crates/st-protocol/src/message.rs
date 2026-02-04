use serde::{Deserialize, Serialize};
use st_domain::model::request::{ProxiedRequest, ProxiedResponse};

#[derive(Debug, Serialize, Deserialize)]
pub enum TunnelMessage {
    // Handshake
    AuthRequest {
        public_key: [u8; 32],
        timestamp: u64,
        signature: Vec<u8>,
    },
    AuthResponse {
        success: bool,
        domain: Option<String>,
        server_public_key: [u8; 32],
        server_signature: Vec<u8>,
    },

    // Data
    HttpRequest(ProxiedRequest),
    HttpResponse(ProxiedResponse),

    // Control
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    Disconnect { reason: String },
}
