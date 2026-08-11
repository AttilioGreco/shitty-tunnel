//! End-to-end tests: a real server, a real client over gRPC, and a real local
//! service, all in-process on ephemeral ports.
//!
//! These exist because every unit below them can be individually correct while
//! the assembled path still loses requests — the request/response correlation
//! across the tunnel has no other coverage.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use shitty_tunnel::app::AppState;
use shitty_tunnel::{client_app::ClientApp, public_handler, tunnel_handler};
use st_infra::config::client::ClientConfig;
use st_infra::config::provider::FileServerConfigProvider;
use st_infra::config::server::ServerSettings;
use st_infra::crypto::auth::Ed25519Authenticator;
use st_infra::crypto::keys::KeyPair;
use st_infra::peer::in_memory::InMemoryPeerRepository;
use st_domain::model::peer::PeerIdentity;
use st_protocol::proto::shitty_tunnel_server::ShittyTunnelServer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

const DOMAIN: &str = "app.example.com";

/// A minimal local service: answers with the request path as the body, so a
/// response can always be traced back to the request that produced it.
async fn spawn_echo_service() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut raw = Vec::new();
                let mut chunk = [0u8; 4096];
                loop {
                    let Ok(n) = sock.read(&mut chunk).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    raw.extend_from_slice(&chunk[..n]);
                    if raw.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }

                let head = String::from_utf8_lossy(&raw).into_owned();
                let request_line = head.lines().next().unwrap_or_default().to_string();
                let mut parts = request_line.split(' ');
                let method = parts.next().unwrap_or("GET").to_string();
                let path = parts.next().unwrap_or("/").to_string();

                // Give the tunnel a chance to interleave in-flight requests.
                if path.contains("slow") {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }

                let body = format!("{method} {path}");
                // `connection: close` keeps the forwarder from reusing a socket
                // this toy server is about to drop. It is stripped before relay.
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\nx-origin: echo\r\nset-cookie: a=1\r\nset-cookie: b=2\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(response.as_bytes()).await;
                let _ = sock.flush().await;
            });
        }
    });

    port
}

struct Harness {
    public_port: u16,
    state: Arc<AppState>,
    server_key: KeyPair,
    tunnel_port: u16,
    local_port: u16,
}

/// Brings up a server whose only enrolled peer is `client_key`.
async fn spawn_server(client_key: &KeyPair) -> Harness {
    let local_port = spawn_echo_service().await;

    let public_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let public_port = public_listener.local_addr().unwrap().port();
    let tunnel_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tunnel_port = tunnel_listener.local_addr().unwrap().port();

    let server_key = KeyPair::generate();
    let peers = InMemoryPeerRepository::new(vec![PeerIdentity {
        public_key: client_key.public_key_bytes(),
        domain: DOMAIN.into(),
    }]);

    let settings = ServerSettings {
        public_port,
        tunnel_port,
        private_key: server_key.private_to_base64(),
        tls: None,
    };

    let state = Arc::new(AppState {
        tunnels: Default::default(),
        authenticator: Arc::new(Ed25519Authenticator::new(
            KeyPair::from_base64(&server_key.private_to_base64()).unwrap(),
            Arc::new(peers),
        )),
        config: Arc::new(FileServerConfigProvider::from_settings(&settings)),
    });

    let public_app = public_handler::router(state.clone());
    tokio::spawn(async move {
        let _ = axum::serve(public_listener, public_app).await;
    });

    let grpc = tunnel_handler::TunnelGrpcService::new(state.clone());
    tokio::spawn(async move {
        let _ = tonic::transport::Server::builder()
            .add_service(ShittyTunnelServer::new(grpc))
            .serve_with_incoming(TcpListenerStream::new(tunnel_listener))
            .await;
    });

    Harness {
        public_port,
        state,
        server_key,
        tunnel_port,
        local_port,
    }
}

fn client_config(h: &Harness, client_key: &KeyPair, basic_auth: &str) -> ClientConfig {
    // Parsed from TOML rather than built field-by-field so the deployed config
    // format stays on the tested path.
    toml::from_str(&format!(
        r#"
        [client]
        server_host = "127.0.0.1"
        server_port = {tunnel_port}
        private_key = "{private_key}"
        server_public_key = "{server_public_key}"

        [local]
        host = "127.0.0.1"
        port = {local_port}
        basic_auth = "{basic_auth}"

        [reconnect]
        enabled = false
        initial_delay_ms = 50
        max_delay_ms = 100
        "#,
        tunnel_port = h.tunnel_port,
        private_key = client_key.private_to_base64(),
        server_public_key = h.server_key.public_to_base64(),
        local_port = h.local_port,
    ))
    .unwrap()
}

fn spawn_client(h: &Harness, client_key: &KeyPair, basic_auth: &str) {
    let app = ClientApp {
        config: client_config(h, client_key, basic_auth),
        key_pair: KeyPair::from_base64(&client_key.private_to_base64()).unwrap(),
        server_public_key: h.server_key.public_key_bytes(),
        event_buffer: None,
    };
    tokio::spawn(async move { app.run().await });
}

/// Waits for the client's tunnel to register, so tests never race the handshake.
async fn await_tunnel(h: &Harness) -> bool {
    for _ in 0..100 {
        if h.state.tunnels.read().await.contains_key(DOMAIN) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

async fn get(port: u16, path: &str) -> reqwest::Response {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
        .get(format!("http://127.0.0.1:{port}{path}"))
        .header("host", DOMAIN)
        .send()
        .await
        .unwrap()
}

// --- tests ------------------------------------------------------------------

#[tokio::test]
async fn a_request_traverses_the_tunnel_and_comes_back_with_its_response() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "");
    assert!(await_tunnel(&h).await, "tunnel never registered");

    let resp = get(h.public_port, "/hello").await;

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("x-origin").unwrap(),
        "echo",
        "application headers must survive the round trip"
    );
    assert_eq!(resp.text().await.unwrap(), "GET /hello");
}

#[tokio::test]
async fn hop_by_hop_headers_from_the_local_service_do_not_reach_the_public_caller() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "");
    assert!(await_tunnel(&h).await);

    let resp = get(h.public_port, "/x").await;

    // The echo service always sends `connection: close`; relaying it would
    // tear down the public keep-alive connection on every request.
    assert!(resp.headers().get("connection").is_none());
}

#[tokio::test]
async fn repeated_set_cookie_headers_survive_the_whole_path() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "");
    assert!(await_tunnel(&h).await);

    let resp = get(h.public_port, "/x").await;

    let cookies: Vec<&str> = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .map(|v| v.to_str().unwrap())
        .collect();
    assert_eq!(cookies, ["a=1", "b=2"]);
}

#[tokio::test]
async fn concurrent_requests_each_receive_their_own_response() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "");
    assert!(await_tunnel(&h).await);

    // /slow finishes last but was issued first: if correlation were positional
    // rather than by request_id, these two responses would be swapped.
    let slow = tokio::spawn(async move { get(h.public_port, "/slow").await.text().await.unwrap() });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let fast = get(h.public_port, "/fast").await.text().await.unwrap();

    assert_eq!(fast, "GET /fast");
    assert_eq!(slow.await.unwrap(), "GET /slow");
}

#[tokio::test]
async fn many_parallel_requests_are_not_cross_wired() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "");
    assert!(await_tunnel(&h).await);

    let port = h.public_port;
    let mut tasks = Vec::new();
    for i in 0..25 {
        tasks.push(tokio::spawn(async move {
            let path = format!("/item/{i}");
            let body = get(port, &path).await.text().await.unwrap();
            (path, body)
        }));
    }

    for task in tasks {
        let (path, body) = task.await.unwrap();
        assert_eq!(body, format!("GET {path}"), "response landed on the wrong request");
    }
}

#[tokio::test]
async fn a_request_for_an_unconnected_domain_is_refused() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "");
    assert!(await_tunnel(&h).await);

    let resp = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("http://127.0.0.1:{}/", h.public_port))
        .header("host", "somebody-else.example.com")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 502);
    assert!(resp.text().await.unwrap().contains("no tunnel"));
}

#[tokio::test]
async fn a_client_whose_key_is_not_enrolled_never_opens_a_tunnel() {
    let enrolled = KeyPair::generate();
    let h = spawn_server(&enrolled).await;

    // Same server, but the client presents a key the server has never seen.
    let stranger = KeyPair::generate();
    spawn_client(&h, &stranger, "");

    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(
        h.state.tunnels.read().await.is_empty(),
        "an unenrolled peer must not be able to register a domain"
    );
}

#[tokio::test]
async fn basic_auth_is_enforced_at_the_far_end_of_the_tunnel() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "user:pass");
    assert!(await_tunnel(&h).await);

    let unauthenticated = get(h.public_port, "/private").await;
    assert_eq!(unauthenticated.status(), 401);
    assert!(unauthenticated.headers().contains_key("www-authenticate"));

    let authorized = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("http://127.0.0.1:{}/private", h.public_port))
        .header("host", DOMAIN)
        .basic_auth("user", Some("pass"))
        .send()
        .await
        .unwrap();
    assert_eq!(authorized.status(), 200);
    assert_eq!(authorized.text().await.unwrap(), "GET /private");
}

#[tokio::test]
async fn a_second_client_cannot_take_over_an_active_domain() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "");
    assert!(await_tunnel(&h).await);

    // Reconnect is disabled, so the rejected client exits instead of looping.
    spawn_client(&h, &client_key, "");
    tokio::time::sleep(Duration::from_millis(300)).await;

    assert_eq!(
        h.state.tunnels.read().await.len(),
        1,
        "the established tunnel must not be displaced"
    );
    assert_eq!(get(h.public_port, "/still-here").await.status(), 200);
}

#[tokio::test]
async fn request_methods_and_bodies_reach_the_local_service() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;
    spawn_client(&h, &client_key, "");
    assert!(await_tunnel(&h).await);

    let resp = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(format!("http://127.0.0.1:{}/submit", h.public_port))
        .header("host", DOMAIN)
        .body("payload")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "POST /submit");
}

#[tokio::test]
async fn a_tunnel_deregisters_when_its_client_goes_away() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;

    let app = ClientApp {
        config: client_config(&h, &client_key, ""),
        key_pair: KeyPair::from_base64(&client_key.private_to_base64()).unwrap(),
        server_public_key: h.server_key.public_key_bytes(),
        event_buffer: None,
    };
    let handle = tokio::spawn(async move { app.run().await });
    assert!(await_tunnel(&h).await);

    handle.abort();

    for _ in 0..100 {
        if h.state.tunnels.read().await.is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("a dead client left its domain registered, blocking reconnection");
}

/// Guards the assumption the harness itself depends on: an unused `HashMap`
/// import here would mean the config path silently stopped carrying headers.
#[tokio::test]
async fn injected_headers_configured_on_the_client_reach_the_public_response() {
    let client_key = KeyPair::generate();
    let h = spawn_server(&client_key).await;

    let mut config = client_config(&h, &client_key, "");
    config.local.add_headers = Some(st_infra::config::client::AddHeaders(HashMap::from([(
        "x-tunnel".to_string(),
        "shitty".to_string(),
    )])));

    let app = ClientApp {
        config,
        key_pair: KeyPair::from_base64(&client_key.private_to_base64()).unwrap(),
        server_public_key: h.server_key.public_key_bytes(),
        event_buffer: None,
    };
    tokio::spawn(async move { app.run().await });
    assert!(await_tunnel(&h).await);

    let resp = get(h.public_port, "/x").await;
    assert_eq!(resp.headers().get("x-tunnel").unwrap(), "shitty");
}
