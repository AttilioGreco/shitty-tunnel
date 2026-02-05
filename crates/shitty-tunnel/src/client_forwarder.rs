use st_domain::model::request::{ProxiedRequest, ProxiedResponse};

pub async fn forward(
    client: &reqwest::Client,
    base_url: &str,
    req: ProxiedRequest,
) -> ProxiedResponse {
    let url = format!("{}{}", base_url, req.uri);
    let method = reqwest::Method::from_bytes(req.method.as_bytes()).unwrap_or(reqwest::Method::GET);

    let start = std::time::Instant::now();

    let mut builder = client.request(method.clone(), &url);

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
        builder = builder.header(key.as_str(), value.as_str());
    }

    if !req.body.is_empty() {
        builder = builder.body(req.body.clone());
    }

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let headers: Vec<(String, String)> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = resp.bytes().await.unwrap_or_default().to_vec();

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
