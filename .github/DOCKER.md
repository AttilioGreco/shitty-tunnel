# 🐳 Docker Image Usage

## Pulling the Image

The Docker image is automatically built and published to GitHub Container Registry on every release.

```bash
# Pull latest version
docker pull ghcr.io/agreco/shittytunnel:latest

# Pull specific version
docker pull ghcr.io/agreco/shittytunnel:v1.0.0
```

## Running the Server

```bash
docker run -d \
  --name shitty-tunnel-server \
  -p 8080:8080 \
  -p 50051:50051 \
  -v /path/to/server.toml:/etc/shittyTunnel/server.toml:ro \
  ghcr.io/agreco/shittytunnel:latest \
  server --config /etc/shittyTunnel/server.toml
```

## Running the Client

```bash
docker run -d \
  --name shitty-tunnel-client \
  -v ~/.config/shittyTunnel.toml:/home/shitty/.config/shittyTunnel.toml:ro \
  ghcr.io/agreco/shittytunnel:latest \
  client --config /home/shitty/.config/shittyTunnel.toml
```

## Docker Compose Example

```yaml
version: '3.8'

services:
  shitty-tunnel-server:
    image: ghcr.io/agreco/shittytunnel:latest
    container_name: shitty-tunnel-server
    ports:
      - "8080:8080"
      - "50051:50051"
    volumes:
      - ./server.toml:/etc/shittyTunnel/server.toml:ro
    command: ["server", "--config", "/etc/shittyTunnel/server.toml"]
    restart: unless-stopped
```

## Multi-Architecture Support

The image supports both `amd64` and `arm64` architectures, so it will work on:
- x86_64 servers
- ARM servers (like AWS Graviton)
- Apple Silicon Macs
- Raspberry Pi (64-bit)

Docker will automatically pull the correct architecture for your platform.

## Security Scanning

All images are automatically scanned for vulnerabilities using Trivy.
Check the Security tab in GitHub for the latest scan results.
