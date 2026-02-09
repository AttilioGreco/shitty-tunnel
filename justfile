# shittyTunnel - Task Runner (just)
# Install just: cargo install just
# Usage: just <task>

set shell := ["bash", "-c"]

# Show available tasks
@default:
    just --list

# Build debug binary
build:
    cargo build --bin shitty-tunnel

# Build release binary
release:
    cargo build --release --bin shitty-tunnel

# Run tests
test:
    cargo test --workspace

# Clean build artifacts
clean:
    cargo clean
    rm -rf build/ target/distrib/

# Install to /usr/local/bin (requires sudo)
install: release
    @echo "Installing shitty-tunnel to /usr/local/bin..."
    sudo install -m 755 target/release/shitty-tunnel /usr/local/bin/shitty-tunnel
    @echo "* Installed: /usr/local/bin/shitty-tunnel"
    shitty-tunnel --help

# cargo-dist: Preview release plan
dist-plan:
    @echo "=== cargo-dist release plan ==="
    cargo dist plan

# cargo-dist: Build multi-platform release
dist-build:
    @echo "=== Building multi-platform release with cargo-dist ==="
    cargo dist build --artifacts all
    @echo ""
    @echo "* Artifacts in: target/distrib/"
    @ls -lh target/distrib/ 2>/dev/null | grep -E '\.(tar\.|zip|installer)' || echo "No artifacts found"

# Build Docker image
docker VERSION="latest":
    docker build -t shitty-tunnel:{{VERSION}} .
    docker tag shitty-tunnel:{{VERSION}} shitty-tunnel:latest

# === Docker ===

# Start all services (with hot-reload)
up:
    @echo "Starting shittyTunnel..."
    docker compose up --build -d
    @echo ""
    @echo "* Environment started!"
    @echo "  - Traefik HTTP:     http://localhost:5000"
    @echo "  - Traefik Dashboard: http://localhost:8081/dashboard/"
    @echo "  - Server HTTP:      http://localhost:8080"
    @echo "  - Server gRPC:      http://localhost:8443"
    @echo ""
    @echo "Test: curl -H \"Host: test.localhost\" http://localhost:5000"
    @echo "Logs: just logs"
    @echo "Stop: just down"
    @echo ""
    @echo "Hot-reload is ACTIVE - edit any .rs file and save!"

# Stop all services
down:
    @echo "Stopping all services..."
    docker compose down

# Restart all services
restart:
    @echo "Restarting all services..."
    docker compose restart

# === Docker Logs ===

# Show all logs
logs:
    docker compose logs -f

# Show server logs
logs-server:
    docker compose logs -f server

# Show client logs
logs-client:
    docker compose logs -f client

# === Docker Cleanup ===

# Stop and remove volumes
clean-docker:
    @echo "Stopping containers and removing volumes..."
    docker compose down -v
    @echo "Cleanup complete"

# Stop, remove volumes AND images
clean-docker-all:
    @echo "Stopping containers, removing volumes AND images..."
    docker compose down -v --rmi all
    @echo "Full cleanup complete"

# === Quick Test ===

# Test the tunnel end-to-end
test-tunnel:
    @echo "Testing tunnel..."
    @sleep 2
    @curl -s -H "Host: test.localhost" http://localhost:5000 | head -n 20
    @echo ""
    @echo "Tunnel is working!"

# === Release Management ===

# Create a new release (updates version, changelog, tags, and pushes)
release-create VERSION:
    @echo "Creating release {{VERSION}}..."
    ./scripts/release.sh {{VERSION}}

# Create release without running tests
release-create-skip-tests VERSION:
    @echo "Creating release {{VERSION}} (skipping tests)..."
    ./scripts/release.sh {{VERSION}} --skip-tests

# Prepare release locally without pushing
release-prepare VERSION:
    @echo "Preparing release {{VERSION}} (no push)..."
    ./scripts/release.sh {{VERSION}} --no-push

# Check what would be released
release-check:
    @echo "Current version: $(grep '^version =' Cargo.toml | head -1 | cut -d'"' -f2)"
    @echo ""
    @echo "Unreleased changes:"
    @sed -n '/^## \[Unreleased\]/,/^## \[/p' CHANGELOG.md | head -n -1

# Undo last release (before push only!)
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
    echo "* Release undone (local only)"
