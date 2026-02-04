# shittyTunnel - Specifiche Tecniche

> Self-hosted HTTP tunnel in Rust. Un'alternativa a ngrok per sviluppatori.

---

## 1. Panoramica

shittyTunnel espone servizi HTTP locali su internet attraverso un server pubblico.

```
Internet                          Server Pubblico                       PC Sviluppatore
                                  ┌─────────────────────┐
                                  │    nginx/caddy       │
  HTTP Request ──────────────────►│  (TLS termination)   │
  Host: dev1.crazylinux.it        │         │            │
                                  │         ▼            │
                                  │  ┌─────────────┐     │         ┌──────────────┐
                                  │  │ shittyServer │◄════════════►│ shittyClient │
                                  │  │ (public_port)│  HTTP/2      │              │
                                  │  │              │  tunnel       │    ┌───────┐ │
                                  │  │(tunnel_port) │  (persistent) │    │:3000  │ │
                                  │  └─────────────┘     │         │    │local  │ │
                                  └─────────────────────┘         │    │server │ │
                                                                   │    └───────┘ │
  ◄─── HTTP Response ◄────────────────────────────────────────────┘              │
                                                                   └──────────────┘
```

**Componenti:**
- **shitty-server**: demone sul server pubblico, riceve traffico HTTP e lo inoltra ai client connessi
- **shitty-client**: demone sul PC dello sviluppatore, riceve traffico dal server e lo inoltra al servizio locale
- **shitty-keygen**: tool per generare coppie di chiavi Ed25519

---

## 2. Architettura - Clean Architecture

Il progetto segue i principi della Clean Architecture con dipendenze che puntano verso l'interno.

```
┌──────────────────────────────────────────────────┐
│                  Binaries                         │
│            (st-server, st-client)                 │
│  ┌────────────────────────────────────────────┐  │
│  │           Infrastructure                    │  │
│  │     (st-infra: crypto, transport,          │  │
│  │      config, http proxy)                    │  │
│  │  ┌──────────────────────────────────────┐  │  │
│  │  │         Protocol                      │  │  │
│  │  │   (st-protocol: framing, codec,      │  │  │
│  │  │    wire messages)                     │  │  │
│  │  │  ┌────────────────────────────────┐  │  │  │
│  │  │  │          Domain                 │  │  │  │
│  │  │  │  (st-domain: models, ports,    │  │  │  │
│  │  │  │   traits, error types)         │  │  │  │
│  │  │  │                                │  │  │  │
│  │  │  │  ZERO dipendenze esterne       │  │  │  │
│  │  │  │  (solo serde per derive)       │  │  │  │
│  │  │  └────────────────────────────────┘  │  │  │
│  │  └──────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────┘
```

### Regola delle dipendenze

- **Domain** non dipende da nulla (solo `serde` per derive)
- **Protocol** dipende solo da Domain
- **Infrastructure** dipende da Domain e Protocol
- **Binaries** dipendono da tutti i layer

---

## 3. Struttura Workspace

```
shittyTunnel/
├── Cargo.toml                      # Workspace root
├── SPEC.md
│
├── crates/
│   ├── st-domain/                  # Layer 1: Domain
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model/
│   │       │   ├── mod.rs
│   │       │   ├── tunnel.rs       # TunnelId, TunnelState, TunnelInfo
│   │       │   ├── peer.rs         # PeerId, PeerIdentity
│   │       │   └── request.rs      # ProxiedRequest, ProxiedResponse
│   │       ├── port/               # Ports = interfacce/traits
│   │       │   ├── mod.rs
│   │       │   ├── auth.rs         # Authenticator trait
│   │       │   ├── tunnel.rs       # TunnelTransport trait
│   │       │   └── proxy.rs        # LocalProxy trait
│   │       └── error.rs            # DomainError enum
│   │
│   ├── st-protocol/                # Layer 2: Protocol (gRPC/protobuf)
│   │   ├── Cargo.toml
│   │   ├── build.rs                # tonic_build per code generation
│   │   ├── proto/
│   │   │   └── tunnel.proto        # Definizione servizio gRPC + messaggi
│   │   └── src/
│   │       ├── lib.rs              # include_proto! + re-export
│   │       └── convert.rs          # From impls proto <-> domain types
│   │
│   ├── st-infra/                   # Layer 3: Infrastructure
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── crypto/
│   │       │   ├── mod.rs
│   │       │   ├── keys.rs         # Ed25519 keypair, keygen, load/save
│   │       │   └── auth.rs         # Implementazione Authenticator
│   │       ├── transport/
│   │       │   ├── mod.rs
│   │       │   ├── server.rs       # Server-side tunnel (accept + manage)
│   │       │   └── client.rs       # Client-side tunnel (connect + reconnect)
│   │       ├── proxy/
│   │       │   ├── mod.rs
│   │       │   └── http_proxy.rs   # reqwest-based local proxy
│   │       └── config/
│   │           ├── mod.rs
│   │           ├── server.rs       # ServerConfig struct
│   │           └── client.rs       # ClientConfig struct
│   │
│   ├── st-server/                  # Binary: server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs             # Entry point, CLI, bootstrap
│   │       ├── app.rs              # ServerApp: orchestrazione use-case
│   │       ├── public_handler.rs   # Handler HTTP pubblico (riceve da nginx)
│   │       └── tunnel_handler.rs   # Handler tunnel (accetta client)
│   │
│   ├── st-client/                  # Binary: client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs             # Entry point, CLI, bootstrap
│   │       ├── app.rs              # ClientApp: orchestrazione use-case
│   │       └── forwarder.rs        # Riceve richieste dal tunnel, forwarda a locale
│   │
│   └── st-keygen/                  # Binary: key generator
│       ├── Cargo.toml
│       └── src/
│           └── main.rs             # Genera keypair Ed25519, stampa su stdout
```

---

## 4. Domain Layer (`st-domain`)

Il cuore del sistema. Nessuna dipendenza esterna significativa.

### 4.1 Models

```rust
// model/tunnel.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TunnelId(pub String); // dominio associato: "dev1.crazylinux.it"

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelState {
    Connecting,
    Authenticating,
    Active,
    Disconnected,
}

#[derive(Debug, Clone)]
pub struct TunnelInfo {
    pub id: TunnelId,
    pub state: TunnelState,
    pub peer: PeerId,
    pub connected_at: Option<u64>,  // unix timestamp
}
```

```rust
// model/peer.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(pub [u8; 32]); // Ed25519 public key

#[derive(Debug, Clone)]
pub struct PeerIdentity {
    pub public_key: [u8; 32],
    pub domain: String,
}
```

```rust
// model/request.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxiedRequest {
    pub request_id: u64,
    pub method: String,
    pub uri: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxiedResponse {
    pub request_id: u64,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}
```

### 4.2 Ports (Traits)

```rust
// port/auth.rs
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Verifica che un peer sia autorizzato e restituisce il dominio associato
    async fn verify_peer(
        &self,
        public_key: &[u8; 32],
        timestamp: u64,
        signature: &[u8; 64],
    ) -> Result<PeerIdentity, DomainError>;

    /// Firma un challenge per dimostrare la propria identita'
    fn sign_challenge(&self, data: &[u8]) -> [u8; 64];

    /// Restituisce la propria chiave pubblica
    fn public_key(&self) -> [u8; 32];
}
```

```rust
// port/tunnel.rs
#[async_trait]
pub trait TunnelTransport: Send + Sync {
    /// Invia una richiesta HTTP attraverso il tunnel
    async fn send_request(&self, req: ProxiedRequest) -> Result<ProxiedResponse, DomainError>;
}
```

```rust
// port/proxy.rs
#[async_trait]
pub trait LocalProxy: Send + Sync {
    /// Inoltra una richiesta al servizio locale
    async fn forward(&self, req: ProxiedRequest) -> Result<ProxiedResponse, DomainError>;
}
```

---

## 5. Protocol Layer (`st-protocol`)

Definisce il formato wire per la comunicazione server↔client.

### 5.1 Wire Format

Ogni messaggio e' incapsulato in un frame:

```
┌─────────────────────────────────────┐
│  Length (4 bytes, big-endian u32)    │
├─────────────────────────────────────┤
│  Payload (N bytes, bincode)         │
└─────────────────────────────────────┘
```

### 5.2 Messaggi

```rust
// message.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum TunnelMessage {
    // --- Handshake ---
    AuthRequest {
        public_key: [u8; 32],
        timestamp: u64,
        signature: [u8; 64],
    },
    AuthResponse {
        success: bool,
        domain: Option<String>,
        server_public_key: [u8; 32],
        server_signature: [u8; 64],  // firma del timestamp per mutual auth
    },

    // --- Data ---
    HttpRequest(ProxiedRequest),
    HttpResponse(ProxiedResponse),

    // --- Control ---
    Ping { nonce: u64 },
    Pong { nonce: u64 },
    Disconnect { reason: String },
}
```

### 5.3 Codec (tokio_util)

```rust
// codec.rs - implementa tokio_util::codec::Decoder + Encoder
// per TunnelMessage con length-prefix framing + bincode
pub struct TunnelCodec;

impl Decoder for TunnelCodec {
    type Item = TunnelMessage;
    type Error = ProtocolError;
    // Legge 4 byte di lunghezza, poi N byte di payload, deserializza con bincode
}

impl Encoder<TunnelMessage> for TunnelCodec {
    type Error = ProtocolError;
    // Serializza con bincode, prepende 4 byte di lunghezza
}
```

---

## 6. Infrastructure Layer (`st-infra`)

### 6.1 Crypto (`crypto/`)

**Libreria: `ed25519-dalek` + `rand`**

```rust
// crypto/keys.rs
pub struct KeyPair {
    signing_key: ed25519_dalek::SigningKey,
}

impl KeyPair {
    pub fn generate() -> Self;
    pub fn from_bytes(secret: &[u8; 32]) -> Self;
    pub fn public_key_bytes(&self) -> [u8; 32];
    pub fn sign(&self, message: &[u8]) -> [u8; 64];
    pub fn to_base64(&self) -> String;       // chiave privata
    pub fn public_to_base64(&self) -> String; // chiave pubblica
    pub fn from_base64(s: &str) -> Result<Self>;
}

pub fn verify_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> bool;
```

**Formato chiavi:** Base64 standard (come WireGuard), 44 caratteri per le chiavi pubbliche.

### 6.2 Transport (`transport/`)

Il tunnel usa HTTP/2 come trasporto. La connessione e' iniziata dal client.

**Meccanismo:**

1. Il client apre una connessione HTTP/2 verso il server (`tunnel_port`)
2. Il client invia un `POST /_tunnel/connect` con body streaming
3. Il server risponde con body streaming
4. Entrambi i body trasportano `TunnelMessage` con framing length-prefix
5. Direzione: server→client = `HttpRequest`, client→server = `HttpResponse`

```rust
// transport/server.rs
pub struct ServerTunnelAcceptor {
    // Accetta connessioni tunnel dai client
    // Per ogni client autenticato, crea un TunnelHandle
}

pub struct TunnelHandle {
    // Canale per inviare richieste al client e ricevere risposte
    pub tx: mpsc::Sender<ProxiedRequest>,
    pub rx: mpsc::Receiver<ProxiedResponse>,
    pub domain: String,
    pub peer_id: PeerId,
}
```

```rust
// transport/client.rs
pub struct ClientTunnelConnector {
    // Gestisce connessione + riconnessione al server
    // Espone un canale per ricevere richieste e inviare risposte
}
```

**Bridge interno (server):**

```
Richiesta HTTP pubblica
        │
        ▼
[axum handler] ──► cerca TunnelHandle per dominio (Host header)
        │
        ▼
[tx.send(ProxiedRequest)] ──► tunnel HTTP/2 stream ──► client
        │
        ▼
[oneshot::Receiver] ◄── attende ProxiedResponse ◄── client
        │
        ▼
[axum response] ──► risposta al chiamante
```

Internamente, ogni richiesta usa un pattern request-response con `tokio::sync::oneshot`:

```rust
struct PendingRequest {
    request_id: u64,
    response_tx: oneshot::Sender<ProxiedResponse>,
}
```

### 6.3 Config (`config/`)

**Server config** (`/etc/shittyTunnel/server.toml`):

```toml
[server]
public_port = 8080          # porta per ricevere HTTP da nginx
tunnel_port = 8443          # porta per connessioni tunnel dai client
private_key = "base64_encoded_server_private_key"

# Opzionale: TLS nativo sul tunnel_port
[server.tls]
enabled = false
cert_path = "/path/to/cert.pem"
key_path = "/path/to/key.pem"

# Un peer per ogni sviluppatore
[[peers]]
public_key = "base64_encoded_client_public_key"
domain = "dev1.crazylinux.it"

[[peers]]
public_key = "base64_encoded_another_client_public_key"
domain = "dev2.crazylinux.it"
```

**Client config** (`~/.config/shittyTunnel.toml`):

```toml
[client]
server_host = "tunnel.crazylinux.it"
server_port = 8443
private_key = "base64_encoded_client_private_key"
server_public_key = "base64_encoded_server_public_key"

[local]
host = "127.0.0.1"
port = 3000

[reconnect]
enabled = true
initial_delay_ms = 1000
max_delay_ms = 30000
# Exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s, 30s, ...
```

### 6.4 HTTP Proxy (`proxy/`)

**Libreria: `reqwest`**

```rust
// proxy/http_proxy.rs
pub struct HttpForwarder {
    client: reqwest::Client,
    target_host: String,
    target_port: u16,
}

impl LocalProxy for HttpForwarder {
    async fn forward(&self, req: ProxiedRequest) -> Result<ProxiedResponse, DomainError> {
        // Costruisce la richiesta verso http://{host}:{port}{uri}
        // Inoltra headers e body
        // Converte la risposta in ProxiedResponse
    }
}
```

---

## 7. Server Application (`st-server`)

### 7.1 Bootstrap

```rust
// main.rs
#[tokio::main]
async fn main() {
    // 1. Parse CLI args (clap)
    // 2. Carica config TOML
    // 3. Inizializza tracing (stdout)
    // 4. Crea ServerApp
    // 5. Avvia server (signal handling per graceful shutdown)
}
```

### 7.2 ServerApp

```rust
// app.rs
pub struct ServerApp {
    config: ServerConfig,
    authenticator: Arc<dyn Authenticator>,
    tunnels: Arc<RwLock<HashMap<String, TunnelHandle>>>, // domain -> tunnel
}

impl ServerApp {
    /// Avvia entrambi i listener (public + tunnel)
    pub async fn run(&self) -> Result<()> {
        tokio::select! {
            r = self.run_public_server() => r,
            r = self.run_tunnel_server() => r,
        }
    }
}
```

### 7.3 Public Handler (riceve HTTP da nginx)

```rust
// public_handler.rs - axum router
async fn handle_request(
    State(app): State<Arc<ServerApp>>,
    req: Request<Body>,
) -> Response<Body> {
    // 1. Estrae Host header
    // 2. Cerca tunnel attivo per quel dominio
    // 3. Se non trovato → 502 Bad Gateway
    // 4. Converte in ProxiedRequest (genera request_id atomico)
    // 5. Invia al tunnel, attende risposta (con timeout)
    // 6. Converte ProxiedResponse in HTTP response
    // 7. Se timeout → 504 Gateway Timeout
}
```

### 7.4 Tunnel Handler (accetta client)

```rust
// tunnel_handler.rs - axum router su tunnel_port
async fn handle_tunnel_connect(
    State(app): State<Arc<ServerApp>>,
    req: Request<Body>,
) -> Response<Body> {
    // 1. Legge primo frame dal body: AuthRequest
    // 2. Verifica con Authenticator
    // 3. Se fallisce → chiude connessione
    // 4. Invia AuthResponse con firma del server
    // 5. Registra TunnelHandle in app.tunnels
    // 6. Loop: legge HttpResponse dal client, matcha con PendingRequest
    // 7. Scrive HttpRequest nel body della risposta streaming
    // 8. Su disconnessione → rimuove da app.tunnels
}
```

---

## 8. Client Application (`st-client`)

### 8.1 Bootstrap

```rust
// main.rs
#[tokio::main]
async fn main() {
    // 1. Parse CLI args (clap)
    // 2. Carica config da ~/.config/shittyTunnel.toml
    // 3. Inizializza tracing
    // 4. Crea ClientApp
    // 5. Avvia con reconnection loop
}
```

### 8.2 ClientApp

```rust
// app.rs
pub struct ClientApp {
    config: ClientConfig,
    authenticator: Arc<dyn Authenticator>,
    forwarder: Arc<dyn LocalProxy>,
}

impl ClientApp {
    pub async fn run(&self) -> Result<()> {
        loop {
            match self.connect_and_serve().await {
                Ok(()) => break,  // graceful shutdown
                Err(e) => {
                    tracing::warn!("tunnel disconnected: {e}, reconnecting...");
                    self.backoff_delay().await;
                }
            }
        }
    }

    async fn connect_and_serve(&self) -> Result<()> {
        // 1. Connessione HTTP/2 al server
        // 2. Handshake (AuthRequest, verifica AuthResponse)
        // 3. Loop: riceve HttpRequest, spawna task per forward
        // 4. Ogni task: forward a locale, invia HttpResponse
    }
}
```

### 8.3 Forwarder

```rust
// forwarder.rs
// Riceve ProxiedRequest dal tunnel
// Usa LocalProxy (reqwest) per inoltrare a localhost:PORT
// Ritorna ProxiedResponse
// Gestisce errori (servizio locale down → 502)
```

---

## 9. Autenticazione - Stile WireGuard

### 9.1 Principio

Come WireGuard: massima semplicita'. Ogni peer ha una coppia Ed25519. Le chiavi pubbliche si scambiano out-of-band.

### 9.2 Setup Iniziale

```bash
# 1. Server genera le sue chiavi
$ shitty-keygen
Private key: kB3VkL9H2a... (salvare in server.toml)
Public key:  mD7xPq2R5c... (dare ai client)

# 2. Ogni client genera le sue chiavi
$ shitty-keygen
Private key: aF5nRt8Y1m... (salvare in ~/.config/shittyTunnel.toml)
Public key:  jK2wXp6S9v... (dare all'admin del server)

# 3. Admin aggiunge il client al server.toml
[[peers]]
public_key = "jK2wXp6S9v..."
domain = "dev1.crazylinux.it"
```

### 9.3 Handshake Flow

```
Client                                    Server
  │                                         │
  │ ──── HTTP/2 POST /_tunnel/connect ────► │
  │                                         │
  │ ──── AuthRequest ─────────────────────► │
  │      { public_key,                      │
  │        timestamp,                       │  Verifica:
  │        signature(timestamp) }           │  1. public_key e' in peers?
  │                                         │  2. timestamp recente? (±30s)
  │                                         │  3. signature valida?
  │                                         │  4. dominio non gia' connesso?
  │                                         │
  │ ◄──── AuthResponse ──────────────────── │
  │       { success: true,                  │
  │         domain: "dev1.crazylinux.it",   │  Client verifica:
  │         server_public_key,              │  1. server_public_key corrisponde?
  │         server_signature(timestamp) }   │  2. signature valida?
  │                                         │
  │ ════ Tunnel Attivo ═══════════════════  │
  │       (bidirectional streaming)         │
```

### 9.4 Protezione Replay

- Il `timestamp` nella AuthRequest impedisce replay attacks (finestra ±30 secondi)
- Il server rifiuta connessioni duplicate per lo stesso dominio

---

## 10. Flusso Completo di una Richiesta

```
1. Browser/Webhook ──► nginx (TLS) ──► shittyServer:8080
                                           │
2. shittyServer estrae Host header         │
   Cerca tunnel per "dev1.crazylinux.it"   │
                                           ▼
3. ProxiedRequest { id: 42, method: "POST", uri: "/webhook", ... }
   ──► serializza bincode ──► length-prefix frame
   ──► HTTP/2 streaming body ──► shittyClient
                                           │
4. shittyClient deserializza               │
   ──► reqwest POST http://127.0.0.1:3000/webhook
   ──► riceve risposta dal server locale   │
                                           ▼
5. ProxiedResponse { id: 42, status: 200, ... }
   ──► serializza ──► frame ──► HTTP/2 stream ──► shittyServer
                                           │
6. shittyServer matcha response id=42      │
   ──► risponde al chiamante originale    │
   ──► nginx ──► Browser/Webhook           ▼
```

---

## 11. Stack Tecnologico e Dipendenze

### Core

| Crate              | Versione | Scopo                                      |
|--------------------|----------|--------------------------------------------|
| `tokio`            | 1.x      | Async runtime (features: full)             |
| `hyper`            | 1.x      | HTTP/1.1 + HTTP/2 low-level                |
| `axum`             | 0.8.x    | HTTP server framework (per entrambe le porte) |
| `reqwest`          | 0.12.x   | HTTP client (client→locale forwarding)     |
| `h2`               | 0.4.x    | HTTP/2 low-level (se serve controllo fine)  |

### Crypto

| Crate              | Versione | Scopo                                      |
|--------------------|----------|--------------------------------------------|
| `ed25519-dalek`    | 2.x      | Ed25519 signatures                         |
| `rand`             | 0.8.x    | Generazione chiavi                         |

### Serializzazione & Config

| Crate              | Versione | Scopo                                      |
|--------------------|----------|--------------------------------------------|
| `serde`            | 1.x      | Framework serializzazione (derive)         |
| `bincode`          | 2.x      | Serializzazione binaria (wire protocol)    |
| `toml`             | 0.8.x    | Parsing config TOML                        |
| `base64`           | 0.22.x   | Encoding chiavi                            |

### Infrastruttura

| Crate              | Versione | Scopo                                      |
|--------------------|----------|--------------------------------------------|
| `tokio-util`       | 0.7.x    | Codec per framing (LengthDelimitedCodec)   |
| `tracing`          | 0.1.x    | Logging strutturato                        |
| `tracing-subscriber` | 0.3.x | Output su stdout                           |
| `clap`             | 4.x      | CLI argument parsing                       |
| `thiserror`        | 2.x      | Error types nelle librerie                 |
| `anyhow`           | 1.x      | Error handling nei binaries                |

### Opzionale (TLS nativo)

| Crate              | Versione | Scopo                                      |
|--------------------|----------|--------------------------------------------|
| `tokio-rustls`     | 0.26.x   | TLS per tunnel_port (se non dietro nginx)  |
| `rustls-pemfile`   | 2.x      | Parsing certificati PEM                    |

---

## 12. CLI Usage

### shitty-keygen

```bash
$ shitty-keygen
Private key: kB3VkL9H2aX7dPm1qR5tN8wY6cF0jS4uZ9eA3bG...
Public key:  mD7xPq2R5cN1fK8hL0jW3vT6yB9sU4aE7gI2dM...
```

### shitty-server

```bash
$ shitty-server --config /etc/shittyTunnel/server.toml

# Output:
# 2024-01-15T10:30:00 INFO  shitty_server: loading config from /etc/shittyTunnel/server.toml
# 2024-01-15T10:30:00 INFO  shitty_server: registered 2 peers
# 2024-01-15T10:30:00 INFO  shitty_server: public HTTP listening on 0.0.0.0:8080
# 2024-01-15T10:30:00 INFO  shitty_server: tunnel listener on 0.0.0.0:8443
# 2024-01-15T10:30:05 INFO  shitty_server: peer connected: dev1.crazylinux.it
```

### shitty-client

```bash
$ shitty-client --config ~/.config/shittyTunnel.toml

# Output:
# 2024-01-15T10:30:05 INFO  shitty_client: connecting to tunnel.crazylinux.it:8443
# 2024-01-15T10:30:05 INFO  shitty_client: authenticated, tunnel active for dev1.crazylinux.it
# 2024-01-15T10:30:05 INFO  shitty_client: forwarding to 127.0.0.1:3000
# 2024-01-15T10:30:10 INFO  shitty_client: POST /webhook -> 200 (45ms)
```

---

## 13. Configurazione Nginx (Esempio)

```nginx
# Per ogni dominio sviluppatore
server {
    listen 443 ssl;
    server_name dev1.crazylinux.it dev2.crazylinux.it;

    ssl_certificate     /path/to/wildcard.pem;
    ssl_certificate_key /path/to/wildcard-key.pem;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

---

## 14. Gestione Errori e Edge Cases

| Scenario                              | Comportamento                                           |
|---------------------------------------|---------------------------------------------------------|
| Client non connesso                   | Server risponde 502 Bad Gateway                        |
| Servizio locale down                  | Client risponde 502, server lo inoltra                 |
| Timeout richiesta (30s default)       | Server risponde 504 Gateway Timeout                    |
| Client si disconnette                 | Server rimuove tunnel, log warning                     |
| Connessione persa                     | Client riprova con exponential backoff                 |
| Chiave non autorizzata                | Server rifiuta handshake, chiude connessione           |
| Dominio gia' connesso                 | Server rifiuta secondo client per stesso dominio       |
| Timestamp troppo vecchio              | Server rifiuta (anti-replay, finestra ±30s)            |
| Body troppo grande                    | Limite configurabile (default 10MB)                    |

---

## 15. Limitazioni v1 e Sviluppi Futuri

### Limitazioni v1
- Solo traffico HTTP (no TCP generico, no WebSocket passthrough)
- Body richiesta/risposta interamente in memoria (no streaming chunked)
- Un solo tunnel per client
- Configurazione statica (richiede restart per aggiungere peer)

### Possibili evoluzioni v2+
- **WebSocket passthrough**: upgrade della connessione tunnel per supportare WS
- **Streaming body**: framing chunked per richieste/risposte grandi
- **TCP generico**: tunnel TCP raw per database, SSH, etc.
- **Hot-reload config**: watch sul file TOML, ricarica senza restart
- **Dashboard web**: stato tunnel, metriche, statistiche
- **Crittografia tunnel**: derivazione chiave di sessione con X25519 DH + ChaCha20-Poly1305
- **Multi-tunnel per client**: un client gestisce piu' domini

---

## 16. Ordine di Implementazione Suggerito

1. **st-domain** - Modelli e traits (fondamenta, zero dipendenze)
2. **st-protocol** - Wire format, codec, framing
3. **st-keygen** - Generazione chiavi Ed25519 (subito testabile)
4. **st-infra/crypto** - Implementazione autenticazione
5. **st-infra/config** - Parsing TOML
6. **st-infra/transport** - Connessione tunnel HTTP/2 (cuore del progetto)
7. **st-server** - Server con entrambi i listener
8. **st-infra/proxy** - Forwarding locale con reqwest
9. **st-client** - Client con reconnection
10. **Test end-to-end** - Server + client + servizio locale fittizio
