# shittyTunnel 🚇

[![Release](https://img.shields.io/github/v/release/agreco/shittyTunnel)](https://github.com/agreco/shittyTunnel/releases)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue)](https://github.com/agreco/shittyTunnel/pkgs/container/shittytunnel)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)

A self-hosted ngrok alternative written in Rust, exposing local services to the internet through secure tunnels.

## Features

- **Ed25519 Authentication** - WireGuard-style mutual authentication with anti-replay protection
- **gRPC Transport** - High-performance bidirectional streaming with protobuf
- **Clean Architecture** - Domain-driven design with clear separation of concerns
- **Docker Support** - Multi-arch images (amd64/arm64) available on ghcr.io
- **Auto-Reconnect** - Client reconnects with exponential backoff
- **Cross-Platform** - Binaries available for Linux, macOS, and Windows

## Quick Start

### Using Docker (Recommended)

```bash
# Pull the latest image
docker pull ghcr.io/agreco/shittytunnel:latest

# Run server
docker run -d \
  -p 8080:8080 -p 50051:50051 \
  -v ./server.toml:/etc/shittyTunnel/server.toml:ro \
  ghcr.io/agreco/shittytunnel:latest \
  server --config /etc/shittyTunnel/server.toml
```

See [Docker documentation](.github/DOCKER.md) for more details.

### Using Pre-built Binaries

Download the latest release for your platform from [GitHub Releases](https://github.com/agreco/shittyTunnel/releases).

### Building from Source

```bash
# Clone the repository
git clone https://github.com/agreco/shittyTunnel.git
cd shittyTunnel

# Build all binaries
cargo build --release

# Binaries will be in target/release/
```

## 📖 Documentation

- [Architecture & Specification](SPEC.md)
- [Docker Usage](.github/DOCKER.md)
- [Configuration Examples](examples/)
- [Release Checklist](.github/RELEASE_CHECKLIST.md)

## Architecture

```
st-domain     → Core business logic (entities, value objects)
st-protocol   → gRPC/Protobuf definitions and conversions
st-infra      → Infrastructure adapters (HTTP, gRPC, crypto)
st-server     → Server binary
st-client     → Client binary
st-keygen     → Key generation utility
```

## Security

- **Ed25519 signatures** for authentication
- **Timestamp-based anti-replay** (±30s window)
- **Mutual authentication** between client and server
- **Automated vulnerability scanning** with Trivy

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
