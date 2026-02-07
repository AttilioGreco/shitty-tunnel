# Docker Test Environment

Ambiente Docker completo per testare shittyTunnel con **Traefik v3**, hot-reload e app di test.

## Quick Start

> **Note:** Questo progetto usa `just` invece di `make` (più idiomatico per Rust)
> Install: `cargo install just`

```bash
# Avvia tutto (hot-reload attivo)
just up

# Test il tunnel
curl -H "Host: test.localhost" http://localhost:5000

# Dovresti vedere la risposta di whoami! 🎉

# Stop
just down
```

## Architettura

```
Browser → Traefik:5000 → Server:8080 → gRPC:8443 → Client → Whoami:80
```

**Componenti:**
- **Traefik v3** - Reverse proxy (sostituisce nginx)
- **shittyTunnel Server** - Gestisce tunnel e routing
- **shittyTunnel Client** - Connette al server e proxa verso app locale
- **Whoami** - App di test HTTP (simula servizio locale dello sviluppatore)

## Porte Esposte

| Servizio | Porta | Descrizione |
|----------|-------|-------------|
| Traefik  | 5000  | 🌍 HTTP pubblico (USA QUESTA!) |
| Traefik  | 8081  | 📊 Dashboard http://localhost:8081/dashboard/ |
| Server   | 8080  | HTTP (riceve da Traefik) |
| Server   | 8443  | gRPC tunnel |

## Comandi Just

```bash
# === Servizi ===
just up                  # Avvia tutto (con hot-reload)
just down                # Stop tutto
just restart             # Restart tutto

# === Logs ===
just logs                # Tutti i servizi
just logs-server         # Solo server
just logs-client         # Solo client

# === Testing ===
just test-tunnel         # Test rapido tunnel

# === Cleanup ===
just clean-docker        # Stop + rimuovi volumi
just clean-docker-all    # Stop + volumi + immagini

# Lista tutti i comandi disponibili
just --list
```

## File di Configurazione

### server.toml
Config del server (porte, chiavi crypto, peers autorizzati):
```toml
[server]
public_port = 8080
tunnel_port = 8443
private_key = "hgrcAHBv0cBkcqrVnwvszkwj9SrxL7YVIRHH02JzNTY="

[[peers]]
public_key = "P1j5jRykDgudgNJNnrJVXHx85W3koAapuyCnCKcq8XM="
domain = "test.localhost"
```

### client.toml
Config del client (URL server, chiavi crypto, local forward):
```toml
[client]
server_host = "http://shitty-tunnel-server:8443"
private_key = "PATS7QnQD+LMGDez8EDi0YCvVx1zjWs1nSu3HjFZ3Fg="
server_public_key = "hgrcAHBv0cBkcqrVnwvszkwj9SrxL7YVIRHH02JzNTY="

[local]
host = "whoami"  # Container hostname
port = 80
```

### traefik.yml
Config statica di Traefik (entrypoints, dashboard).

### traefik-dynamic.yml
Config dinamica di Traefik (routing rules).

## Hot-Reload

**cargo-watch** ricompila automaticamente al cambio dei file:

```bash
# Avvia tutto
just up

# Modifica un file .rs
vim ../crates/st-server/src/app.rs

# Salva → cargo-watch ricompila e riavvia automaticamente! 🚀
```

**Volumi montati:**
- `.:/app:cached` - Codice sorgente
- `cargo-cache` - Registry Cargo (velocizza rebuild)
- `target-cache` - Build artifacts (velocizza rebuild)

## Test End-to-End

### Test Rapido

```bash
just test-tunnel
```

### Test Manuale

```bash
# 1. Verifica container running
docker compose ps

# 2. Verifica logs
just logs-client  # Dovresti vedere "tunnel active"

# 3. Test HTTP attraverso il tunnel
curl -H "Host: test.localhost" http://localhost:5000

# Dovresti vedere la risposta di whoami con:
# - Hostname: shitty-whoami-test
# - IP: <container_ip>
# - Headers ricevuti
```

## Traefik Dashboard

Apri: **http://localhost:8081/dashboard/**

Verifica:
- **Entrypoint** `web` su porta 5000
- **Router** `tunnel-router` attivo
- **Service** `shitty-server` con backend 8080 (verde = healthy)

## Chiavi Crypto (Ed25519)

**IMPORTANTE:** Le chiavi devono corrispondere!

| Componente | File | Campo | Valore |
|------------|------|-------|--------|
| Server | server.toml | `private_key` | hgrcAHBv... |
| Client | client.toml | `server_public_key` | hgrcAHBv... (STESSO!) |
| Server | server.toml | `[[peers]].public_key` | P1j5jRyk... |
| Client | client.toml | `private_key` | PATS7QnQ... (genera P1j5jRyk...) |

**Generare nuove chiavi:**
```bash
cargo run --bin shitty-tunnel -- keygen

# Output:
# Private key: kB3VkL9H2a...
# Public key:  mD7xPq2R5c...
```

## Troubleshooting

### ❌ Container non si avvia

```bash
# Controlla logs
just logs

# Rebuild completo
just clean-docker-all
just up
```

### ❌ Client non si connette

```bash
# Verifica chiavi
grep "server_public_key" client.toml
grep "private_key" server.toml
# Devono corrispondere!

# Controlla logs del server
just logs-server
# Cerca "authentication failed"
```

### ❌ 502 Bad Gateway

```bash
# Verifica client connesso
just logs-client
# Dovresti vedere "authenticated" o "tunnel active"

# Verifica whoami running
docker compose ps whoami

# Testa whoami direttamente
docker compose exec shitty-tunnel-client-dev wget -O- http://whoami:80
```

### ❌ Hot-reload non funziona

```bash
# Verifica container running
docker compose ps
# Dovresti vedere: shitty-server, shitty-client

# Verifica cargo-watch nei logs
just logs-server
# Dovresti vedere: "[Running 'cargo run ...']"

# Restart
just restart
```

## Note

- **Hot-reload sempre attivo** - cargo-watch di default
- **Primo build**: 2-3 minuti (dipendenze Rust)
- **Rebuild successivi**: 5-10 secondi (grazie cache incrementale)
- **Network Docker**: `shitty-tunnel-net` condiviso tra container
- **Dashboard Traefik**: In insecure mode per test locali (no auth)

## Link Utili

- **Traefik Dashboard**: http://localhost:8081/dashboard/
- **Test Tunnel**: `curl -H "Host: test.localhost" http://localhost:5000`
- **Lista comandi**: `just --list`

---

💡 **Tip**: Usa `just test-tunnel` per verificare velocemente che tutto funzioni!
