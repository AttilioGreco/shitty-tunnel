# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.16] - 2026-02-26

## [0.1.15] - 2026-02-25

## [0.1.14] - 2026-02-25

## [0.1.14] - 2026-02-25

## [0.1.13] - 2026-02-25

## [0.1.12] - 2026-02-25

## [0.1.11] - 2026-02-25

## [0.1.10] - 2026-02-25

## [0.1.9] - 2026-02-24

## [0.1.7] - 2026-02-24

## [0.1.6] - 2026-02-23

## [0.1.5] - 2026-02-23

## [0.1.4] - 2026-02-23

## [0.1.3] - 2026-02-23

## [0.1.2] - 2026-02-10

## [0.1.1] - 2026-02-09

## [0.1.0] - 2026-02-09

### Added
- Docker Compose setup for local development and testing
- Complete Docker setup documentation (DOCKER_SETUP.md)

### Fixed
- Fixed URL parsing in client: now correctly handles full URLs like `https://host.com:443`
- Server host config now supports both full URLs and separate host+port configuration
- Made `server_port` optional when using full URL in `server_host`

## [0.0.1] - 2024-02-05

### Fixed
- Fixed client URL construction: auto-detect http/https based on port (443 = https, other = http)
- Support explicit schema in server_host config (e.g., `https://tunnel.example.com`)
- Fixed duplicate protocol bug (`http://https://...`)

## [0.1.2] - 2024-02-05

### Fixed
- Switched reqwest to rustls-tls for musl static builds (no OpenSSL dependency)
- Added libssl-dev and pkg-config to CI dependencies for native TLS builds

## [0.1.1] - 2024-02-05

### Fixed
- Added protobuf-compiler dependency to CI workflows
- Fixed cargo-dist system dependencies configuration

## [0.1.0] - 2024-02-05

### Added
- Initial release
- gRPC bidirectional streaming tunnel
- Ed25519 mutual authentication
- Auto-reconnect with exponential backoff
- Unified CLI with server/client/keygen subcommands
- Clean Architecture implementation
- Docker support
- Examples and documentation
- Multi-platform builds (Linux x64/ARM64, macOS Intel/ARM, Windows)
- cargo-dist release automation

