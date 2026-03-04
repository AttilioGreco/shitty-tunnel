# shittyTunnel - Task Runner
# Install just: cargo install just
# Usage: just <task>

set shell := ["bash", "-c"]

# Show available tasks
@default:
    just --list

# === Build ===

# Build debug binary
build:
    cargo build --bin shitty-tunnel

# Force fresh embedded frontend assets on next Rust build
frontend-fresh:
    @echo "Forcing fresh frontend embed build (removing frontend/dist)..."
    rm -rf frontend/dist

# Build release binary
release: frontend-fresh
    ST_REQUIRE_FRONTEND=1 cargo build --release --bin shitty-tunnel

# Run all workspace tests
test:
    cargo test --workspace

# Clean build artifacts
clean:
    cargo clean
    rm -rf build/ target/distrib/

# === Install ===

# Build release and install to /usr/local/bin (requires sudo)
install: release
    @echo "Installing shitty-tunnel to /usr/local/bin..."
    sudo install -m 755 target/release/shitty-tunnel /usr/local/bin/shitty-tunnel
    @echo "Installed: /usr/local/bin/shitty-tunnel"
    shitty-tunnel --help

# Install server systemd unit (system-wide, requires sudo)
install-systemd-server:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Installing shitty-tunnel-server.service..."
    sudo mkdir -p /etc/shittyTunnel
    sudo tee /etc/systemd/system/shitty-tunnel-server.service > /dev/null <<'EOF'
    [Unit]
    Description=shittyTunnel Server
    After=network-online.target
    Wants=network-online.target

    [Service]
    Type=simple
    ExecStart=/usr/local/bin/shitty-tunnel server --config %h/.config/shittyTunnel.toml
    Restart=on-failure
    RestartSec=5
    NoNewPrivileges=true
    ProtectSystem=strict
    ProtectHome=read-only

    [Install]
    WantedBy=multi-user.target
    EOF
    sudo systemctl daemon-reload
    echo ""
    echo "Installed: /etc/systemd/system/shitty-tunnel-server.service"
    echo ""
    echo "Next steps:"
    echo "  1. Edit ~/.config/shittyTunnel.toml (see examples/server.toml)"
    echo "  2. sudo systemctl enable --now shitty-tunnel-server"
    echo "  3. sudo journalctl -u shitty-tunnel-server -f"

# Install client systemd user unit (per-user, no sudo)
install-systemd-client:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p ~/.config/systemd/user
    cat > ~/.config/systemd/user/shitty-tunnel-client.service <<'EOF'
    [Unit]
    Description=shittyTunnel Client
    After=network-online.target
    Wants=network-online.target

    [Service]
    Type=simple
    ExecStart=/usr/local/bin/shitty-tunnel client --config %h/.config/shittyTunnel.toml
    Restart=on-failure
    RestartSec=5
    Environment=RUST_LOG=info

    [Install]
    WantedBy=default.target
    EOF
    systemctl --user daemon-reload
    echo ""
    echo "Installed: ~/.config/systemd/user/shitty-tunnel-client.service"
    echo ""
    echo "Next steps:"
    echo "  1. Edit ~/.config/shittyTunnel.toml (see examples/client.toml)"
    echo "  2. systemctl --user enable --now shitty-tunnel-client"
    echo "  3. journalctl --user -u shitty-tunnel-client -f"
    echo "  4. loginctl enable-linger $USER  (to keep running after logout)"

# Uninstall server systemd unit
uninstall-systemd-server:
    @echo "Stopping and removing shitty-tunnel-server.service..."
    -@sudo systemctl stop shitty-tunnel-server
    -@sudo systemctl disable shitty-tunnel-server
    @sudo rm -f /etc/systemd/system/shitty-tunnel-server.service
    @sudo systemctl daemon-reload
    @echo "Removed."

# Uninstall client systemd user unit
uninstall-systemd-client:
    @echo "Stopping and removing shitty-tunnel-client.service..."
    -@systemctl --user stop shitty-tunnel-client
    -@systemctl --user disable shitty-tunnel-client
    @rm -f ~/.config/systemd/user/shitty-tunnel-client.service
    @systemctl --user daemon-reload
    @echo "Removed."

# === cargo-dist ===

# Preview release plan
dist-plan:
    @echo "=== cargo-dist release plan ==="
    cargo dist plan

# Build multi-platform release
dist-build: frontend-fresh
    @echo "=== Building multi-platform release with cargo-dist ==="
    ST_REQUIRE_FRONTEND=1 cargo dist build --artifacts all
    @echo ""
    @echo "Artifacts in: target/distrib/"
    @ls -lh target/distrib/ 2>/dev/null | grep -E '\.(tar\.|zip|installer)' || echo "No artifacts found"

# === Docker ===

# Build Docker image
docker VERSION="latest":
    docker build -t shitty-tunnel:{{VERSION}} .
    docker tag shitty-tunnel:{{VERSION}} shitty-tunnel:latest

# Start dev environment (Docker Compose)
up:
    @echo "Starting shittyTunnel dev environment..."
    docker compose up --build -d
    @echo ""
    @echo "Environment started:"
    @echo "  Traefik HTTP:      http://localhost:5000"
    @echo "  Traefik Dashboard: http://localhost:8081/dashboard/"
    @echo "  Server HTTP:       http://localhost:8080"
    @echo "  Server gRPC:       http://localhost:8443"
    @echo ""
    @echo "Test: curl -H \"Host: test.localhost\" http://localhost:5000"
    @echo "Logs: just logs"
    @echo "Stop: just down"

# Stop dev environment
down:
    docker compose down

# Restart dev environment
restart:
    docker compose restart

# Show all logs (Docker Compose)
logs:
    docker compose logs -f

# Show server logs
logs-server:
    docker compose logs -f server

# Show client logs
logs-client:
    docker compose logs -f client

# Stop and remove volumes
clean-docker:
    docker compose down -v

# Stop, remove volumes and images
clean-docker-all:
    docker compose down -v --rmi all

# === Quick Test ===

# Test the tunnel end-to-end (requires dev environment running)
test-tunnel:
    @echo "Testing tunnel..."
    @sleep 2
    @curl -s -H "Host: test.localhost" http://localhost:5000 | head -n 20
    @echo ""
    @echo "Tunnel is working!"

# === Release ===

# Create a new release (version, changelog, tag, push)
release-create VERSION:
    ./scripts/release.sh {{VERSION}}

# Create release without running tests
release-create-skip-tests VERSION:
    ./scripts/release.sh {{VERSION}} --skip-tests

# Prepare release locally without pushing
release-prepare VERSION:
    ./scripts/release.sh {{VERSION}} --no-push

# Show current version and unreleased changes
release-check:
    @echo "Current version: $(grep '^version =' Cargo.toml | head -1 | cut -d'"' -f2)"
    @echo ""
    @echo "Unreleased changes:"
    @sed -n '/^## \[Unreleased\]/,/^## \[/p' CHANGELOG.md | head -n -1

# Undo last release (local only, before push!)
release-undo:
    @echo "Undoing last release..."
    @LAST_TAG=$$(git describe --tags --abbrev=0 2>/dev/null || echo "no-tag"); \
    if [ "$$LAST_TAG" = "no-tag" ]; then \
        echo "No tags found"; \
        exit 1; \
    fi; \
    echo "Deleting tag: $$LAST_TAG"; \
    git tag -d "$$LAST_TAG"; \
    echo "Resetting last commit"; \
    git reset --hard HEAD~1; \
    echo "Release undone (local only)"
