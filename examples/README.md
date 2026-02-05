# shittyTunnel - Esempio di configurazione

Questa directory contiene configurazioni di esempio per provare shittyTunnel in locale.

## Setup rapido (test locale)

### 1. Genera le chiavi

```bash
# Genera chiavi per il server
shitty-tunnel keygen
# Output:
# Private key: axjhCqieuY3cU6qpRA48FSjKlojaH5+Q5kjm5aLwdfc=
# Public key:  P1j5jRykDgudgNJNnrJVXHx85W3koAapuyCnCKcq8XM=

# Genera chiavi per il client
shitty-tunnel keygen
# Output:
# Private key: PATS7QnQD+LMGDez8EDi0YCvVx1zjWs1nSu3HjFZ3Fg=
# Public key:  ZlsOtT/650gpT8XrPzNJOT4yyuJoYfngRnbRBBHknYE=
```

### 2. Configura il server

Crea `/etc/shittyTunnel/server.toml` (o copia `examples/server.toml`):

```toml
[server]
public_port = 8080
tunnel_port = 8443
private_key = "PRIVATE_KEY_DEL_SERVER"

[[peers]]
public_key = "PUBLIC_KEY_DEL_CLIENT"
domain = "dev1.example.com"
```

**Importante:**
- `server.private_key` = chiave privata generata per il server
- `peers[].public_key` = chiave **pubblica** del client (scambio out-of-band)

### 3. Configura il client

Crea `~/.config/shittyTunnel.toml` (o copia `examples/client.toml`):

```toml
[client]
server_host = "localhost"  # o IP/hostname del server
server_port = 8443
private_key = "PRIVATE_KEY_DEL_CLIENT"
server_public_key = "PUBLIC_KEY_DEL_SERVER"

[local]
host = "127.0.0.1"
port = 3000  # porta del tuo servizio locale
```

**Importante:**
- `client.private_key` = chiave privata generata per il client
- `client.server_public_key` = chiave **pubblica** del server (scambio out-of-band)

### 4. Avvia un servizio locale (test)

```bash
# Esempio con Python
cd /tmp && python3 -m http.server 3000
```

### 5. Avvia il server

```bash
shitty-tunnel server --config /etc/shittyTunnel/server.toml
# Output:
# INFO loaded config from /etc/shittyTunnel/server.toml
# INFO registered 1 peers
# INFO public HTTP listening on 0.0.0.0:8080
# INFO gRPC tunnel listening on 0.0.0.0:8443
```

### 6. Avvia il client

```bash
shitty-tunnel client --config ~/.config/shittyTunnel.toml
# Output:
# INFO connecting to localhost:8443
# INFO connected to localhost:8443
# INFO authenticated, tunnel active for dev1.example.com
# INFO forwarding to 127.0.0.1:3000
```

### 7. Testa il tunnel

```bash
# Richiesta HTTP al server sulla porta pubblica
curl -H "Host: dev1.example.com" http://localhost:8080/

# Il server forwarda al client, che forwarda a localhost:3000
# Dovresti vedere la risposta del tuo servizio locale
```

---

## Setup produzione

### Architettura

```
Internet
  │
  ▼
┌─────────────────────┐
│  nginx/caddy/ingress │  ← TLS termination
│  (*.example.com)     │
└──────┬───────┬───────┘
       │       │
       │       │ :8443 (gRPC tunnel)
       ▼       │
  ┌─────────┐ │
  │ shitty- │◄┘
  │ tunnel  │
  │ server  │
  │  :8080  │ (HTTP pubblico)
  └─────────┘
       ▲
       │ gRPC/HTTP2
       │
  ┌─────────┐
  │ shitty- │
  │ tunnel  │──► localhost:3000
  │ client  │    (servizio locale)
  └─────────┘
  PC Sviluppatore
```

### Configurazione nginx

```nginx
# HTTP pubblico (riceve traffico da internet)
upstream shitty_public {
    server 127.0.0.1:8080;
}

# gRPC tunnel (client si connettono qui)
upstream shitty_tunnel {
    server 127.0.0.1:8443;
}

# Wildcard per tutti i domini sviluppatori
server {
    listen 443 ssl http2;
    server_name *.example.com;

    ssl_certificate /path/to/wildcard.pem;
    ssl_certificate_key /path/to/wildcard-key.pem;

    # Traffico HTTP pubblico → shitty-tunnel public port
    location / {
        proxy_pass http://shitty_public;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}

# Endpoint tunnel per i client
server {
    listen 443 ssl http2;
    server_name tunnel.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    # gRPC tunnel endpoint
    location / {
        grpc_pass grpc://shitty_tunnel;
        grpc_set_header Host $host;
    }
}
```

### Docker Compose (esempio)

```yaml
version: '3.8'

services:
  shitty-server:
    image: shitty-tunnel:latest
    command: ["server", "--config", "/config/server.toml"]
    volumes:
      - ./server.toml:/config/server.toml:ro
    ports:
      - "8080:8080"  # public HTTP (nginx upstream)
      - "8443:8443"  # gRPC tunnel
    restart: unless-stopped
```

### Aggiungere un nuovo sviluppatore

1. Lo sviluppatore genera le sue chiavi:
   ```bash
   shitty-tunnel keygen
   ```

2. Lo sviluppatore invia la **chiave pubblica** all'admin

3. Admin aggiunge il peer al `server.toml`:
   ```toml
   [[peers]]
   public_key = "chiave_pubblica_dello_sviluppatore"
   domain = "dev-nome.example.com"
   ```

4. Admin riavvia il server (o hot-reload se implementato)

5. Admin invia allo sviluppatore:
   - Chiave pubblica del server
   - Hostname tunnel (`tunnel.example.com`)
   - Dominio assegnato (`dev-nome.example.com`)

6. Sviluppatore configura `~/.config/shittyTunnel.toml` e avvia il client

---

## Troubleshooting

### Client non si connette

```
ERROR tunnel error: connection refused
```

**Soluzione:** Verifica che il server sia in ascolto sulla `tunnel_port` e che sia raggiungibile.

### Autenticazione fallita

```
ERROR tunnel error: Unauthenticated: auth failed
```

**Cause:**
- Chiavi sbagliate (verifica `client.private_key` e `client.server_public_key`)
- Client non autorizzato (la sua public key non è in `server.toml`)
- Clock skew > 30s (sincronizza NTP su client e server)

### 502 Bad Gateway

```
curl -H "Host: dev1.example.com" http://localhost:8080/
502 Bad Gateway
```

**Cause:**
- Client non connesso (verifica log client)
- Servizio locale down (verifica `local.host:local.port`)
- Dominio sbagliato nell'Host header

### Tunnel si disconnette continuamente

**Soluzione:**
- Verifica log lato server per errori
- Controlla firewall/ingress timeout (gRPC long-lived connection)
- Se dietro nginx, aumenta `grpc_read_timeout` e `grpc_send_timeout`

---

## Sicurezza

- Le chiavi private **non devono mai** essere committate in git
- Usa `.gitignore` per `*.toml` nelle directory con configurazioni reali
- Considera secret manager per ambienti produzione (Vault, k8s secrets, etc.)
- Il server deve validare che ogni dominio sia usato da un solo client contemporaneamente (già implementato)
- Usa TLS/mTLS per il tunnel in produzione (nginx/ingress o tonic TLS)

---

## Performance

- Il tunnel gRPC/HTTP2 supporta multiplexing nativo (più richieste sullo stesso stream)
- Backpressure automatico (flow control HTTP/2)
- Timeout richiesta: 30s (modificabile in `public_handler.rs`)
- Body limit: 10MB (modificabile nel codec protobuf)


# systemd
Per eseguire shitty-tunnel come servizio di sistema, crea i seguenti file di unità systemd.

## Server

`/etc/systemd/system/shitty-tunnel-server.service`

```ini
[Unit]
Description=shitty-tunnel Server
After=network.target
[Service]
ExecStart=/usr/local/bin/shitty-tunnel server --config /etc/shittyTunnel/server.toml
Restart=on-failure
[Install]
WantedBy=multi-user.target
```

## Client

`/etc/systemd/system/shitty-tunnel-client.service`

```ini
[Unit]
Description=shitty-tunnel Client
After=network.target
[Service]
ExecStart=/usr/local/bin/shitty-tunnel client --config /home/USERNAME/.config/shittyTunnel.toml
Restart=on-failure
[Install]
WantedBy=multi-user.target
```