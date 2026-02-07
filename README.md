# shittyTunnel

**Self-hosted HTTP tunnel - ngrok alternative in Rust**

Expose your local HTTP services to the internet through a public server. Perfect for:
- Webhook development (GitHub, Stripe, Telegram bots, etc.)
- Sharing local dev environments
- Testing mobile apps against localhost
- IoT device access

## Features

- 🔒 **Ed25519 authentication** - WireGuard-style mutual auth with timestamp anti-replay
- 🚀 **gRPC bidirectional streaming** - HTTP/2 multiplexing, native backpressure
- 🔄 **Auto-reconnect** - Exponential backoff with configurable delays
- 🏗️ **Clean Architecture** - Domain-driven design, testable, maintainable
- 🐳 **Docker ready** - Single static binary or container
- 🌍 **Multi-tenant** - One server, multiple developers/domains

## Quick Start

### Installation

#### Homebrew (macOS/Linux)

```bash
brew install yourusername/tap/shitty-tunnel
```

#### Shell script (Linux/macOS)

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/yourusername/shittyTunnel/releases/latest/download/shitty-tunnel-installer.sh | sh
```

#### Pre-built binaries

Download from [GitHub Releases](https://github.com/yourusername/shittyTunnel/releases):
- Linux x64 (glibc/musl) + ARM64
- macOS Intel + Apple Silicon
- Windows x64

#### From source

```bash
cargo install --git https://github.com/yourusername/shittyTunnel
```

### Generate keys

```bash
# Server
shitty-tunnel keygen
# Save private key to server config, share public key with clients

# Client
shitty-tunnel keygen
# Save private key to client config, share public key with admin
```

### Configure

**Server** (`/etc/shittyTunnel/server.toml`):

```toml
[server]
public_port = 8080
tunnel_port = 8443
private_key = "YOUR_SERVER_PRIVATE_KEY"

[[peers]]
public_key = "CLIENT_PUBLIC_KEY"
domain = "dev1.example.com"
```

**Client** (`~/.config/shittyTunnel.toml`):

```toml
[client]
server_host = "tunnel.example.com"
server_port = 8443
private_key = "YOUR_CLIENT_PRIVATE_KEY"
server_public_key = "SERVER_PUBLIC_KEY"

[local]
host = "127.0.0.1"
port = 3000
```

### Run

```bash
# Server
shitty-tunnel server

# Client
shitty-tunnel client

# Test
curl -H "Host: dev1.example.com" http://your-server:8080/
```

## Docker Development Setup

Quickly test shittyTunnel locally with Docker Compose + Traefik:

> **Note:** This project uses [`just`](https://github.com/casey/just) instead of `make`
> Install: `cargo install just`

```bash
# Start everything (Traefik + Server + Client + Test app)
just up

# Test the tunnel
curl -H "Host: test.localhost" http://localhost:5000

# View logs
just logs

# Stop
just down
```

See [docker/README.md](docker/README.md) for complete Docker setup guide.

## Development

This project uses [`just`](https://github.com/casey/just) as task runner:

```bash
# Install just
cargo install just

# Show all available tasks
just --list

# Common tasks
just build           # Build debug
just release         # Build release
just test            # Run tests
just up              # Start Docker environment (hot-reload)
```

## Documentation

- [Examples & Setup Guide](examples/README.md)
- [Docker Setup](docker/README.md)
- [Architecture Spec](SPEC.md)

## Architecture

```
Internet → nginx/ingress → shittyServer:8080 (HTTP)
                         ↓
                    :8443 (gRPC tunnel)
                         ↓
                    shittyClient → localhost:3000
```

- **Domain layer**: Pure business logic, zero dependencies
- **Protocol layer**: gRPC/protobuf bidirectional streaming
- **Infrastructure layer**: Ed25519 crypto, config, HTTP proxy
- **Binaries**: Unified CLI with subcommands

## Security

- **Ed25519 signatures** - Public-key cryptography, no shared secrets
- **Timestamp anti-replay** - ±30s window prevents replay attacks
- **Mutual authentication** - Both server and client verify each other
- **One domain per client** - Server enforces exclusive tunnel ownership
- **TLS recommended** - Use nginx/ingress for TLS termination or tonic native TLS

## Performance

- **HTTP/2 multiplexing** - Multiple requests on single connection
- **Backpressure** - Flow control prevents overwhelming local service
- **Timeout**: 30s per request (configurable)
- **Body limit**: 10MB (configurable)

## Comparison with ngrok

| Feature | shittyTunnel | ngrok |
|---------|--------------|-------|
| Self-hosted | ✅ | ❌ |
| Open source | ✅ | ❌ |
| Multi-tenant | ✅ | ✅ |
| Custom domains | ✅ | ✅ (paid) |
| Ed25519 auth | ✅ | ❌ |
| gRPC transport | ✅ | ❌ |
| Static binary | ✅ | ✅ |

## License

MIT OR Apache-2.0

## Contributing

See [examples/README.md](examples/README.md) for development setup.
