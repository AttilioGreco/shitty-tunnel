use serde::Serialize;

const MAX_BODY_SIZE: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct RequestEvent {
    pub id: u64,
    pub timestamp: String,
    pub offset_ms: f64,
    pub request: RequestData,
    pub response: Option<ResponseData>,
    pub duration_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequestData {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub body_size: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResponseData {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub body_size: usize,
    pub truncated: bool,
}

impl RequestData {
    pub fn from_parts(method: &str, path: &str, headers: &[(String, String)], body: &[u8]) -> Self {
        use base64::Engine;
        let body_size = body.len();
        let truncated = body_size > MAX_BODY_SIZE;
        let body_slice = if truncated { &body[..MAX_BODY_SIZE] } else { body };
        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: headers.to_vec(),
            body: base64::prelude::BASE64_STANDARD.encode(body_slice),
            body_size,
            truncated,
        }
    }
}

impl ResponseData {
    pub fn from_parts(status: u16, headers: &[(String, String)], body: &[u8]) -> Self {
        use base64::Engine;
        let body_size = body.len();
        let truncated = body_size > MAX_BODY_SIZE;
        let body_slice = if truncated { &body[..MAX_BODY_SIZE] } else { body };
        Self {
            status,
            headers: headers.to_vec(),
            body: base64::prelude::BASE64_STANDARD.encode(body_slice),
            body_size,
            truncated,
        }
    }
}
