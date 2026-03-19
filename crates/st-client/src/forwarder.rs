use st_domain::model::request::{ProxiedRequest, ProxiedResponse};
use st_infra::config::client::{AddHeaders, RemoveHeaders};

pub async fn forward(
    client: &reqwest::Client,
    base_url: &str,
    req: ProxiedRequest,
    add_headers: Option<&AddHeaders>,
    remove_headers: Option<&RemoveHeaders>,
    max_body_size: usize,
) -> ProxiedResponse {
    let url = format!("{}{}", base_url, req.uri);
    let method = reqwest::Method::from_bytes(req.method.as_bytes()).unwrap_or(reqwest::Method::GET);

    let start = std::time::Instant::now();

    let mut builder = client.request(method.clone(), &url);

    let remove_set: Vec<String> = remove_headers
        .map(|r| r.names.iter().map(|n| n.to_lowercase()).collect())
        .unwrap_or_default();

    for (key, value) in &req.headers {
        let lower = key.to_lowercase();
        // Skip hop-by-hop headers
        if lower == "host"
            || lower == "connection"
            || lower == "transfer-encoding"
            || lower == "keep-alive"
        {
            continue;
        }
        if remove_set.contains(&lower) {
            continue;
        }
        builder = builder.header(key.as_str(), value.as_str());
    }

    // Inject/overwrite configured headers
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
            let mut headers: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .filter(|(k, _)| !remove_set.contains(&k.to_lowercase()))
                .collect();

            // Inject/overwrite configured headers into response
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

            let body = resp.bytes().await.unwrap_or_default().to_vec();
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
