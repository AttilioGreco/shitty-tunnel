use base64::Engine;
use st_domain::model::request::{ProxiedRequest, ProxiedResponse};
use st_infra::config::client::{AddHeaders, RemoveHeaders};

/// Headers that belong to a single hop and must never be relayed, plus the
/// framing headers hyper has already resolved on our behalf: it de-chunks the
/// body but leaves `transfer-encoding: chunked` in place, and re-frames length
/// itself downstream.
///
/// `content-encoding` is deliberately absent: reqwest is built without the
/// gzip/brotli/deflate/zstd features, so the body reaches us still encoded and
/// the header is accurate. Stripping it would hand raw gzip to the caller
/// labelled as plaintext. Both behaviours are pinned in tests/passthrough.rs.
fn is_hop_by_hop(lower_name: &str) -> bool {
    matches!(
        lower_name,
        "connection"
            | "keep-alive"
            | "transfer-encoding"
            | "content-length"
            | "upgrade"
            | "te"
            | "trailer"
            | "proxy-authenticate"
            | "proxy-authorization"
    )
}

pub async fn forward(
    client: &reqwest::Client,
    base_url: &str,
    req: ProxiedRequest,
    basic_auth: &str,
    add_headers: Option<&AddHeaders>,
    remove_headers: Option<&RemoveHeaders>,
    max_body_size: usize,
) -> ProxiedResponse {
    // Check basic auth if configured
    if !basic_auth.is_empty() {
        let authorized = req
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
            .and_then(|(_, v)| v.strip_prefix("Basic "))
            .and_then(|encoded| {
                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .ok()
            })
            .and_then(|decoded| String::from_utf8(decoded).ok())
            .is_some_and(|creds| creds == basic_auth);

        if !authorized {
            return ProxiedResponse {
                request_id: req.request_id,
                status: 401,
                headers: vec![
                    ("content-type".into(), "text/plain".into()),
                    ("www-authenticate".into(), "Basic realm=\"shittyTunnel\"".into()),
                ],
                body: b"Unauthorized".to_vec(),
            };
        }
    }

    let url = format!("{}{}", base_url, req.uri);
    // Falling back to GET would silently turn an unparseable method into a
    // different, successful request against the local service.
    let Ok(method) = reqwest::Method::from_bytes(req.method.as_bytes()) else {
        return ProxiedResponse {
            request_id: req.request_id,
            status: 400,
            headers: vec![("content-type".to_string(), "text/plain".to_string())],
            body: format!("invalid request method: {}", req.method).into_bytes(),
        };
    };

    let start = std::time::Instant::now();

    let mut builder = client.request(method.clone(), &url);

    let remove_set: Vec<String> = remove_headers
        .map(|r| r.names.iter().map(|n| n.to_lowercase()).collect())
        .unwrap_or_default();

    for (key, value) in &req.headers {
        let lower = key.to_lowercase();
        // `host` is rebuilt by reqwest for the local target.
        if lower == "host" || is_hop_by_hop(&lower) {
            continue;
        }
        if remove_set.contains(&lower) {
            continue;
        }
        builder = builder.header(key.as_str(), value.as_str());
    }

    // Inject/overwrite configured request headers
    if let Some(add) = add_headers {
        for (key, value) in &add.0 {
            builder = builder.header(key.as_str(), value.as_str());
        }
    }

    if !req.body.is_empty() {
        builder = builder.body(req.body.clone());
    }

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            // HeaderName is always lowercase, so no normalisation is needed here.
            let mut headers: Vec<(String, String)> = resp
                .headers()
                .iter()
                .filter(|(k, _)| {
                    !is_hop_by_hop(k.as_str()) && !remove_set.iter().any(|r| r == k.as_str())
                })
                // A non-UTF8 value cannot be relayed; forwarding it as "" would
                // turn a malformed header into a plausible empty one.
                .filter_map(|(k, v)| Some((k.to_string(), v.to_str().ok()?.to_string())))
                .collect();

            // Inject/overwrite configured response headers
            if let Some(add) = add_headers {
                for (key, value) in &add.0 {
                    let lower = key.to_lowercase();
                    headers.retain(|(k, _)| k.to_lowercase() != lower);
                    headers.push((key.clone(), value.clone()));
                }
            }

            // Early reject if content-length exceeds the configured limit
            let content_length = resp
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            if content_length > max_body_size {
                return ProxiedResponse {
                    request_id: req.request_id,
                    status: 502,
                    headers: vec![("content-type".to_string(), "text/plain".to_string())],
                    body: format!("response body too large: {content_length} bytes (limit: {max_body_size})").into_bytes(),
                };
            }

            // A failure here means the local service died mid-body. Defaulting to
            // an empty body would relay its status (typically 200) with no
            // content, which reads as a successful empty page.
            let body = match resp.bytes().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    tracing::error!("{} {} -> body read failed: {e}", method, req.uri);
                    return ProxiedResponse {
                        request_id: req.request_id,
                        status: 502,
                        headers: vec![("content-type".to_string(), "text/plain".to_string())],
                        body: format!("local service response truncated: {e}").into_bytes(),
                    };
                }
            };
            if body.len() > max_body_size {
                return ProxiedResponse {
                    request_id: req.request_id,
                    status: 502,
                    headers: vec![("content-type".to_string(), "text/plain".to_string())],
                    body: format!("response body too large: {} bytes (limit: {max_body_size})", body.len()).into_bytes(),
                };
            }

            let elapsed = start.elapsed();
            tracing::info!("{} {} -> {} ({:.0?})", method, req.uri, status, elapsed);

            ProxiedResponse {
                request_id: req.request_id,
                status,
                headers,
                body,
            }
        }
        Err(e) => {
            tracing::error!("{} {} -> error: {e}", method, req.uri);

            ProxiedResponse {
                request_id: req.request_id,
                status: 502,
                headers: vec![("content-type".to_string(), "text/plain".to_string())],
                body: format!("local service error: {e}").into_bytes(),
            }
        }
    }
}
