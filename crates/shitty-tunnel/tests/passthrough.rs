//! reqwest is built without gzip/brotli/deflate/zstd features, so it must hand
//! back response bytes exactly as the local service framed them. The forwarder
//! relies on that: if this ever changes, stripping or keeping `content-encoding`
//! flips from correct to corrupting, and every compressed response breaks.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Gzip magic bytes — not valid gzip, so a decompressing client would error or
/// mangle them rather than return them untouched.
const COMPRESSED_BODY: &[u8] = &[0x1f, 0x8b, 0x08, 0x00, 0xde, 0xad, 0xbe, 0xef];

async fn serve_once(headers: &'static str, body: &'static [u8]) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();

        let mut buf = [0u8; 2048];
        while let Ok(n) = sock.read(&mut buf).await {
            if n == 0 || buf[..n].windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        sock.write_all(headers.as_bytes()).await.unwrap();
        sock.write_all(body).await.unwrap();
        sock.flush().await.unwrap();
    });

    format!("http://{addr}/")
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
}

#[tokio::test]
async fn reqwest_does_not_decompress_response_bodies() {
    let url = serve_once(
        "HTTP/1.1 200 OK\r\n\
         content-type: text/html\r\n\
         content-encoding: gzip\r\n\
         content-length: 8\r\n\
         \r\n",
        COMPRESSED_BODY,
    )
    .await;

    let resp = client().get(&url).send().await.unwrap();

    assert_eq!(
        resp.headers().get("content-encoding").map(|v| v.as_bytes()),
        Some(&b"gzip"[..]),
        "content-encoding must survive: the body is still compressed"
    );

    let body = resp.bytes().await.unwrap();
    assert_eq!(
        body.as_ref(),
        COMPRESSED_BODY,
        "body must be byte-identical; reqwest must not decode it"
    );
}

#[tokio::test]
async fn reqwest_hides_chunked_framing_from_the_forwarder() {
    let url = serve_once(
        "HTTP/1.1 200 OK\r\n\
         content-type: text/plain\r\n\
         transfer-encoding: chunked\r\n\
         \r\n",
        b"5\r\nhello\r\n0\r\n\r\n",
    )
    .await;

    let resp = client().get(&url).send().await.unwrap();

    let te = resp
        .headers()
        .get("transfer-encoding")
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned());
    let body = resp.bytes().await.unwrap();

    // hyper de-chunks the body but leaves the header in place: forwarding both
    // verbatim tells the far client to expect chunk framing that is no longer
    // there, which desyncs the connection. The forwarder must drop the header.
    assert_eq!(te.as_deref(), Some("chunked"));
    assert_eq!(body.as_ref(), b"hello");
}
