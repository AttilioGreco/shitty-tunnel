//! Behavioural tests for the client-side forwarder.
//!
//! The local service is a raw TCP responder rather than an HTTP framework: the
//! forwarder's job is to relay framing faithfully, so the tests need to control
//! the exact bytes on the wire and to inspect the exact request head that the
//! forwarder produced.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use shitty_tunnel::client_forwarder::forward;
use st_domain::model::request::{ProxiedRequest, ProxiedResponse};
use st_infra::config::client::{AddHeaders, RemoveHeaders};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// A local service that always answers with the same canned bytes and records
/// every request head it received.
struct TestService {
    addr: SocketAddr,
    seen: Arc<Mutex<Vec<String>>>,
}

impl TestService {
    async fn spawn(response: &'static [u8]) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorder = seen.clone();

        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    return;
                };
                let recorder = recorder.clone();

                tokio::spawn(async move {
                    let mut raw = Vec::new();
                    let mut chunk = [0u8; 4096];

                    // Read the head, then whatever body content-length announces.
                    loop {
                        let Ok(n) = sock.read(&mut chunk).await else {
                            return;
                        };
                        if n == 0 {
                            return;
                        }
                        raw.extend_from_slice(&chunk[..n]);

                        let text = String::from_utf8_lossy(&raw).into_owned();
                        let Some(head_end) = text.find("\r\n\r\n") else {
                            continue;
                        };
                        let want = content_length(&text[..head_end]);
                        if raw.len() >= head_end + 4 + want {
                            recorder.lock().await.push(text);
                            break;
                        }
                    }

                    let _ = sock.write_all(response).await;
                    let _ = sock.flush().await;
                });
            }
        });

        Self { addr, seen }
    }

    fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    async fn last_request(&self) -> String {
        self.seen
            .lock()
            .await
            .last()
            .cloned()
            .expect("local service received no request")
    }

    async fn request_count(&self) -> usize {
        self.seen.lock().await.len()
    }
}

fn content_length(head: &str) -> usize {
    head.lines()
        .find_map(|l| {
            let (name, value) = l.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())?
        })
        .unwrap_or(0)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

fn request(method: &str, headers: Vec<(&str, &str)>) -> ProxiedRequest {
    ProxiedRequest {
        request_id: 1,
        method: method.into(),
        uri: "/resource".into(),
        headers: headers
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        body: vec![],
    }
}

/// Runs the forwarder with the defaults every test but the targeted one wants.
async fn forward_default(service: &TestService, req: ProxiedRequest) -> ProxiedResponse {
    forward(
        &client(),
        &service.base_url(),
        req,
        "",
        None,
        None,
        1024 * 1024,
    )
    .await
}

fn header<'a>(resp: &'a ProxiedResponse, name: &str) -> Option<&'a str> {
    resp.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

const OK_RESPONSE: &[u8] =
    b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 2\r\n\r\nhi";

// --- basic auth -------------------------------------------------------------

#[tokio::test]
async fn basic_auth_challenges_a_request_with_no_credentials() {
    let service = TestService::spawn(OK_RESPONSE).await;

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![]),
        "user:pass",
        None,
        None,
        1024,
    )
    .await;

    assert_eq!(resp.status, 401);
    assert_eq!(
        header(&resp, "www-authenticate"),
        Some("Basic realm=\"shittyTunnel\""),
        "a challenge is required or browsers will not prompt"
    );
    assert_eq!(
        service.request_count().await,
        0,
        "the local service must never see an unauthenticated request"
    );
}

#[tokio::test]
async fn basic_auth_rejects_wrong_credentials() {
    let service = TestService::spawn(OK_RESPONSE).await;
    let wrong = format!(
        "Basic {}",
        base64_encode(b"user:wrong")
    );

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![("authorization", &wrong)]),
        "user:pass",
        None,
        None,
        1024,
    )
    .await;

    assert_eq!(resp.status, 401);
    assert_eq!(service.request_count().await, 0);
}

#[tokio::test]
async fn basic_auth_accepts_correct_credentials() {
    let service = TestService::spawn(OK_RESPONSE).await;
    let good = format!("Basic {}", base64_encode(b"user:pass"));

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![("authorization", &good)]),
        "user:pass",
        None,
        None,
        1024,
    )
    .await;

    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, b"hi");
}

#[tokio::test]
async fn basic_auth_rejects_a_malformed_authorization_header() {
    let service = TestService::spawn(OK_RESPONSE).await;

    for bad in ["Basic !!!not-base64!!!", "Bearer token", "Basic"] {
        let resp = forward(
            &client(),
            &service.base_url(),
            request("GET", vec![("authorization", bad)]),
            "user:pass",
            None,
            None,
            1024,
        )
        .await;

        assert_eq!(resp.status, 401, "must reject {bad:?}");
    }
    assert_eq!(service.request_count().await, 0);
}

#[tokio::test]
async fn an_empty_basic_auth_setting_disables_the_check() {
    let service = TestService::spawn(OK_RESPONSE).await;

    let resp = forward_default(&service, request("GET", vec![])).await;

    assert_eq!(resp.status, 200);
}

// --- response framing -------------------------------------------------------

#[tokio::test]
async fn a_compressed_body_keeps_its_content_encoding() {
    // reqwest is built without decompression features, so the body arrives
    // still encoded: dropping the header here would corrupt every gzip response.
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ncontent-type: text/html\r\ncontent-encoding: gzip\r\ncontent-length: 4\r\n\r\n\x1f\x8b\x08\x00",
    )
    .await;

    let resp = forward_default(&service, request("GET", vec![])).await;

    assert_eq!(header(&resp, "content-encoding"), Some("gzip"));
    assert_eq!(resp.body, vec![0x1f, 0x8b, 0x08, 0x00]);
}

#[tokio::test]
async fn chunked_framing_is_not_advertised_after_hyper_reassembles_the_body() {
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ntransfer-encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
    )
    .await;

    let resp = forward_default(&service, request("GET", vec![])).await;

    assert_eq!(resp.body, b"hello");
    assert!(
        header(&resp, "transfer-encoding").is_none(),
        "relaying chunked framing over an already-assembled body desyncs the caller"
    );
}

#[tokio::test]
async fn hop_by_hop_response_headers_are_not_relayed() {
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 2\r\nconnection: keep-alive\r\nkeep-alive: timeout=5\r\nx-app: keep\r\n\r\nhi",
    )
    .await;

    let resp = forward_default(&service, request("GET", vec![])).await;

    for stripped in ["connection", "keep-alive", "content-length"] {
        assert!(
            header(&resp, stripped).is_none(),
            "{stripped} belongs to the local hop only"
        );
    }
    assert_eq!(
        header(&resp, "x-app"),
        Some("keep"),
        "application headers must survive"
    );
}

#[tokio::test]
async fn repeated_response_headers_are_all_preserved() {
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nset-cookie: a=1\r\nset-cookie: b=2\r\n\r\nhi",
    )
    .await;

    let resp = forward_default(&service, request("GET", vec![])).await;

    let cookies: Vec<&str> = resp
        .headers
        .iter()
        .filter(|(k, _)| k == "set-cookie")
        .map(|(_, v)| v.as_str())
        .collect();
    assert_eq!(cookies, ["a=1", "b=2"], "dropping one silently loses a session");
}

#[tokio::test]
async fn the_status_code_is_relayed_verbatim() {
    for status in [201u16, 301, 404, 418, 500] {
        let response: &'static [u8] = Box::leak(
            format!("HTTP/1.1 {status} X\r\ncontent-length: 0\r\n\r\n").into_bytes().into_boxed_slice(),
        );
        let service = TestService::spawn(response).await;

        let resp = forward_default(&service, request("GET", vec![])).await;
        assert_eq!(resp.status, status);
    }
}

// --- request side -----------------------------------------------------------

#[tokio::test]
async fn the_public_host_header_does_not_leak_to_the_local_service() {
    let service = TestService::spawn(OK_RESPONSE).await;

    forward_default(
        &service,
        request("GET", vec![("host", "public.example.com")]),
    )
    .await;

    let sent = service.last_request().await;
    assert!(
        !sent.to_lowercase().contains("public.example.com"),
        "the local service must be addressed by its own authority:\n{sent}"
    );
}

#[tokio::test]
async fn hop_by_hop_request_headers_are_not_relayed() {
    let service = TestService::spawn(OK_RESPONSE).await;

    forward_default(
        &service,
        request(
            "GET",
            vec![
                ("connection", "upgrade"),
                ("keep-alive", "timeout=5"),
                ("x-app", "keep"),
            ],
        ),
    )
    .await;

    let sent = service.last_request().await.to_lowercase();
    assert!(!sent.contains("keep-alive: timeout=5"), "{sent}");
    assert!(sent.contains("x-app: keep"), "{sent}");
}

#[tokio::test]
async fn the_request_body_reaches_the_local_service_intact() {
    let service = TestService::spawn(OK_RESPONSE).await;

    let mut req = request("POST", vec![("content-type", "application/json")]);
    req.body = b"{\"hello\":\"world\"}".to_vec();

    let resp = forward_default(&service, req).await;

    assert_eq!(resp.status, 200);
    let sent = service.last_request().await;
    assert!(sent.contains("{\"hello\":\"world\"}"), "{sent}");
    assert!(sent.starts_with("POST /resource"), "{sent}");
}

#[tokio::test]
async fn the_uri_including_its_query_string_is_forwarded() {
    let service = TestService::spawn(OK_RESPONSE).await;

    let mut req = request("GET", vec![]);
    req.uri = "/search?q=rust&page=2".into();

    forward_default(&service, req).await;

    let sent = service.last_request().await;
    assert!(sent.starts_with("GET /search?q=rust&page=2 "), "{sent}");
}

// --- header manipulation ----------------------------------------------------

#[tokio::test]
async fn configured_headers_are_injected_into_the_request_and_the_response() {
    let service = TestService::spawn(OK_RESPONSE).await;
    let add = AddHeaders(HashMap::from([(
        "x-injected".to_string(),
        "yes".to_string(),
    )]));

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![]),
        "",
        Some(&add),
        None,
        1024,
    )
    .await;

    assert!(service.last_request().await.to_lowercase().contains("x-injected: yes"));
    assert_eq!(header(&resp, "x-injected"), Some("yes"));
}

#[tokio::test]
async fn an_injected_response_header_overwrites_the_local_services_value() {
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nx-app: original\r\n\r\nhi",
    )
    .await;
    let add = AddHeaders(HashMap::from([(
        "x-app".to_string(),
        "override".to_string(),
    )]));

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![]),
        "",
        Some(&add),
        None,
        1024,
    )
    .await;

    let values: Vec<&str> = resp
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("x-app"))
        .map(|(_, v)| v.as_str())
        .collect();
    assert_eq!(values, ["override"], "the original must not remain alongside");
}

#[tokio::test]
async fn configured_headers_are_stripped_from_both_directions() {
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nx-secret: leaked\r\n\r\nhi",
    )
    .await;
    let remove = RemoveHeaders {
        names: vec!["X-Secret".into(), "Cookie".into()],
    };

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![("cookie", "session=abc")]),
        "",
        None,
        Some(&remove),
        1024,
    )
    .await;

    assert!(
        header(&resp, "x-secret").is_none(),
        "removal must be case-insensitive against the wire name"
    );
    assert!(!service.last_request().await.to_lowercase().contains("session=abc"));
}

// --- limits and failure modes -----------------------------------------------

#[tokio::test]
async fn an_oversized_content_length_is_rejected_before_the_body_is_read() {
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: 5000\r\n\r\n",
    )
    .await;

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![]),
        "",
        None,
        None,
        100,
    )
    .await;

    assert_eq!(resp.status, 502);
    assert!(String::from_utf8_lossy(&resp.body).contains("too large"));
}

#[tokio::test]
async fn an_oversized_body_without_content_length_is_still_rejected() {
    // Chunked responses carry no length, so the limit must also hold after read.
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ntransfer-encoding: chunked\r\n\r\n1e\r\naaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n0\r\n\r\n",
    )
    .await;

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![]),
        "",
        None,
        None,
        10,
    )
    .await;

    assert_eq!(resp.status, 502);
    assert!(String::from_utf8_lossy(&resp.body).contains("too large"));
}

#[tokio::test]
async fn a_body_at_the_limit_is_allowed_through() {
    let service = TestService::spawn(OK_RESPONSE).await;

    let resp = forward(
        &client(),
        &service.base_url(),
        request("GET", vec![]),
        "",
        None,
        None,
        2,
    )
    .await;

    assert_eq!(resp.status, 200, "the limit must be inclusive");
    assert_eq!(resp.body, b"hi");
}

#[tokio::test]
async fn an_unparseable_method_is_refused_instead_of_becoming_a_get() {
    let service = TestService::spawn(OK_RESPONSE).await;

    let resp = forward_default(&service, request("BAD METHOD", vec![])).await;

    assert_eq!(resp.status, 400);
    assert_eq!(
        service.request_count().await,
        0,
        "a malformed method must not reach the local service at all"
    );
}

#[tokio::test]
async fn uncommon_but_valid_methods_are_forwarded_unchanged() {
    for method in ["PUT", "DELETE", "PATCH", "OPTIONS", "PROPFIND"] {
        let service = TestService::spawn(OK_RESPONSE).await;

        let resp = forward_default(&service, request(method, vec![])).await;

        assert_eq!(resp.status, 200);
        assert!(
            service.last_request().await.starts_with(method),
            "{method} must not be rewritten"
        );
    }
}

#[tokio::test]
async fn an_unreachable_local_service_surfaces_as_a_bad_gateway() {
    // Port 1 on loopback: reserved and not listening.
    let resp = forward(
        &client(),
        "http://127.0.0.1:1",
        request("GET", vec![]),
        "",
        None,
        None,
        1024,
    )
    .await;

    assert_eq!(resp.status, 502);
    assert!(String::from_utf8_lossy(&resp.body).contains("local service error"));
}

#[tokio::test]
async fn a_connection_dropped_mid_body_does_not_become_an_empty_success() {
    // Announces 100 bytes, sends 4, then closes: the old code relayed this as
    // a 200 with an empty body, which renders as a blank page.
    let service = TestService::spawn(
        b"HTTP/1.1 200 OK\r\ncontent-type: text/html\r\ncontent-length: 100\r\n\r\nshor",
    )
    .await;

    let resp = forward_default(&service, request("GET", vec![])).await;

    assert_eq!(
        resp.status, 502,
        "a truncated body must not be reported as success"
    );
}

fn base64_encode(raw: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(raw)
}
