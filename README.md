# shittyTunnel 🚇

[![Release](https://img.shields.io/github/v/release/agreco/shittyTunnel)](https://github.com/agreco/shittyTunnel/releases)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue)](https://github.com/agreco/shittyTunnel/pkgs/container/shittytunnel)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)

A self-hosted tunnel solution, for exposing your local services to the internet.

## Features

- **Ed25519 Authentication** - WireGuard-style mutual authentication with anti-replay protection
- **gRPC Transport** - High-performance bidirectional streaming with protobuf, But works with a simple ingress without TCP configuration.
- **Clean Architecture** - Domain-driven design with clear separation of concerns
- **Docker Support** - Multi-arch images (amd64/arm64) available on ghcr.io
- **Auto-Reconnect** - Client reconnects with exponential backoff
- **Cross-Platform** - Binaries available for Linux, macOS, and Windows

## Disclaimer
This project was born out of personal frustration.

Tired of relying on paid services, I decided to experiment with a different approach: unleashing an AI agent to build a simple piece of software to solve my problem.
As a result, this project was written almost entirely by an LLM, and for the most part without line-by-line human supervision.

After a few hours of work—alternating prompts, tests, and small adjustments—the program started working far better than I had expected. At that point, I decided to invest more time into it: refining prompts, making requests more precise, and refactoring the overall architecture. In my experience, guiding an LLM with a clear (even if initially a bit messy) structure helps mitigate many common context-related issues.

After further iterations and some manual refactoring of the CI and release process, I’m satisfied with the final result. That said, if you dig into the code, you may find a bit of everything: questionable choices, creative solutions, and inevitable compromises.

Don’t expect too much from this project. But if you have the time and curiosity to try it out, let me know what you think—you might be surprised.
And if you’re a skilled Rust developer, pull requests and improvement suggestions are more than welcome.


## Quick Start

### Using Docker (Recommended)

```bash
# Pull the latest image
docker pull ghcr.io/attiliogreco/shitty-tunnel:latest

# Run server
docker run -d \
  -p 8080:8080 -p 50051:50051 \
  -v ./server.toml:/etc/shittyTunnel/server.toml:ro \
  ghcr.io/attiliogreco/shitty-tunnel:latest \
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
