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

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ProxiedRequest {
        ProxiedRequest {
            request_id: 42,
            method: "POST".into(),
            uri: "/api/items?q=1&q=2".into(),
            headers: vec![
                ("content-type".into(), "application/json".into()),
                ("x-trace".into(), "abc".into()),
            ],
            body: b"{\"a\":1}".to_vec(),
        }
    }

    fn response() -> ProxiedResponse {
        ProxiedResponse {
            request_id: 42,
            status: 201,
            headers: vec![
                ("set-cookie".into(), "a=1".into()),
                ("set-cookie".into(), "b=2".into()),
            ],
            body: vec![0x00, 0xff, 0x1f, 0x8b],
        }
    }

    #[test]
    fn request_survives_a_round_trip_through_the_wire_type() {
        let original = request();
        let restored: ProxiedRequest = proto::HttpRequest::from(original.clone()).into();

        assert_eq!(restored.request_id, original.request_id);
        assert_eq!(restored.method, original.method);
        assert_eq!(restored.uri, original.uri, "query string must be preserved");
        assert_eq!(restored.headers, original.headers);
        assert_eq!(restored.body, original.body);
    }

    #[test]
    fn response_survives_a_round_trip_through_the_wire_type() {
        let original = response();
        let restored: ProxiedResponse = proto::HttpResponse::from(original.clone()).into();

        assert_eq!(restored.request_id, original.request_id);
        assert_eq!(restored.status, original.status);
        assert_eq!(
            restored.headers, original.headers,
            "repeated headers must not be collapsed into a map"
        );
        assert_eq!(
            restored.body, original.body,
            "binary bodies must not be re-encoded as text"
        );
    }

    #[test]
    fn header_order_is_preserved() {
        // Set-Cookie ordering is semantically meaningful to clients.
        let original = response();
        let wire = proto::HttpResponse::from(original);

        assert_eq!(wire.headers[0].value, "a=1");
        assert_eq!(wire.headers[1].value, "b=2");
    }

    #[test]
    fn an_empty_body_and_no_headers_round_trip_cleanly() {
        let original = ProxiedRequest {
            request_id: 0,
            method: "GET".into(),
            uri: "/".into(),
            headers: vec![],
            body: vec![],
        };
        let restored: ProxiedRequest = proto::HttpRequest::from(original.clone()).into();

        assert!(restored.headers.is_empty());
        assert!(restored.body.is_empty());
        assert_eq!(restored.uri, "/");
    }

    #[test]
    fn status_narrowing_is_lossless_for_real_http_codes() {
        for status in [100u16, 200, 301, 404, 418, 500, 599] {
            let wire = proto::HttpResponse::from(ProxiedResponse {
                request_id: 1,
                status,
                headers: vec![],
                body: vec![],
            });
            let restored: ProxiedResponse = wire.into();
            assert_eq!(restored.status, status);
        }
    }

    #[test]
    fn a_request_id_at_the_top_of_the_range_is_not_truncated() {
        // IDs come from a process-lifetime counter; the wire type must carry
        // the full u64 so correlation cannot alias after a long uptime.
        let wire = proto::HttpRequest::from(ProxiedRequest {
            request_id: u64::MAX,
            method: "GET".into(),
            uri: "/".into(),
            headers: vec![],
            body: vec![],
        });

        let restored: ProxiedRequest = wire.into();
        assert_eq!(restored.request_id, u64::MAX);
    }
}
