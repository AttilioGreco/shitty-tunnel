# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/yourusername/shittyTunnel/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/yourusername/shittyTunnel/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/yourusername/shittyTunnel/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/yourusername/shittyTunnel/releases/tag/v0.1.0
