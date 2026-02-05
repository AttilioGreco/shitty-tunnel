.PHONY: help build release clean test docker install dist-build dist-plan

VERSION ?= 0.1.0

help:
	@echo "shittyTunnel - Makefile targets"
	@echo ""
	@echo "Development:"
	@echo "  make build          - Build debug binary"
	@echo "  make test           - Run tests"
	@echo "  make clean          - Clean build artifacts"
	@echo ""
	@echo "Release (cargo):"
	@echo "  make release        - Build release binary (native)"
	@echo "  make install        - Install to /usr/local/bin"
	@echo ""
	@echo "Release (cargo-dist - multi-platform):"
	@echo "  make dist-plan      - Preview release artifacts"
	@echo "  make dist-build     - Build all platform binaries + installers"
	@echo ""
	@echo "Docker:"
	@echo "  make docker         - Build Docker image"
	@echo ""

build:
	cargo build --bin shitty-tunnel

release:
	cargo build --release --bin shitty-tunnel

dist-plan:
	@echo "=== cargo-dist release plan ==="
	@dist plan

dist-build:
	@echo "=== Building multi-platform release with cargo-dist ==="
	@dist build --artifacts all
	@echo ""
	@echo "✓ Artifacts in: target/distrib/"
	@ls -lh target/distrib/ 2>/dev/null | grep -E '(\.tar\.|\.zip|installer)' || echo "No artifacts found"

docker:
	docker build -t shitty-tunnel:$(VERSION) .
	docker tag shitty-tunnel:$(VERSION) shitty-tunnel:latest

install:
	@if [ ! -f target/release/shitty-tunnel ]; then \
		echo "Building release binary first..."; \
		cargo build --release --bin shitty-tunnel; \
	fi
	@echo "Installing shitty-tunnel to /usr/local/bin..."
	sudo install -m 755 target/release/shitty-tunnel /usr/local/bin/shitty-tunnel
	@echo "✓ Installed: /usr/local/bin/shitty-tunnel"
	@shitty-tunnel --help

clean:
	cargo clean
	rm -rf build/ target/distrib/

test:
	cargo test --workspace

.DEFAULT_GOAL := help
