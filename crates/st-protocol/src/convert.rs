use st_domain::model::request::{ProxiedRequest, ProxiedResponse};

use crate::proto;

impl From<ProxiedRequest> for proto::HttpRequest {
    fn from(r: ProxiedRequest) -> Self {
        proto::HttpRequest {
            request_id: r.request_id,
            method: r.method,
            uri: r.uri,
            headers: r
                .headers
                .into_iter()
                .map(|(name, value)| proto::HttpHeader { name, value })
                .collect(),
            body: r.body,
        }
    }
}

impl From<proto::HttpRequest> for ProxiedRequest {
    fn from(r: proto::HttpRequest) -> Self {
        ProxiedRequest {
            request_id: r.request_id,
            method: r.method,
            uri: r.uri,
            headers: r.headers.into_iter().map(|h| (h.name, h.value)).collect(),
            body: r.body,
        }
    }
}

impl From<ProxiedResponse> for proto::HttpResponse {
    fn from(r: ProxiedResponse) -> Self {
        proto::HttpResponse {
            request_id: r.request_id,
            status: r.status as u32,
            headers: r
                .headers
                .into_iter()
                .map(|(name, value)| proto::HttpHeader { name, value })
                .collect(),
            body: r.body,
        }
    }
}

impl From<proto::HttpResponse> for ProxiedResponse {
    fn from(r: proto::HttpResponse) -> Self {
        ProxiedResponse {
            request_id: r.request_id,
            status: r.status as u16,
            headers: r.headers.into_iter().map(|h| (h.name, h.value)).collect(),
            body: r.body,
        }
    }
}
