#!/usr/bin/env bash
set -euo pipefail

# shittyTunnel Client Installer
# Usage: curl -sSL https://raw.githubusercontent.com/AttilioGreco/shitty-tunnel/main/scripts/install-client.sh | bash
#
# Environment variables:
#   SHITTY_TUNNEL_VERSION   - specific version to install (default: latest)
#   SHITTY_TUNNEL_INSTALL_DIR - install directory (default: ~/.local/bin)

REPO="AttilioGreco/shitty-tunnel"
BINARY_NAME="shitty-tunnel"
INSTALL_DIR="${SHITTY_TUNNEL_INSTALL_DIR:-$HOME/.local/bin}"
CONFIG_DIR="$HOME/.config"
CONFIG_FILE="$CONFIG_DIR/shittyTunnel.toml"
SERVICE_DIR="$HOME/.config/systemd/user"

info()  { echo "  [+] $*"; }
warn()  { echo "  [!] $*" >&2; }
error() { echo "  [✗] $*" >&2; exit 1; }

detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)  os="unknown-linux" ;;
    Darwin) os="apple-darwin" ;;
    *)      error "Unsupported OS: $os" ;;
  esac

  case "$arch" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *)             error "Unsupported architecture: $arch" ;;
  esac

  # Prefer musl on Linux for static linking
  if [ "$os" = "unknown-linux" ]; then
    os="unknown-linux-musl"
  fi

  echo "${arch}-${os}"
}

get_latest_version() {
  if [ -n "${SHITTY_TUNNEL_VERSION:-}" ]; then
    echo "$SHITTY_TUNNEL_VERSION"
    return
  fi

  local version
  version="$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4)"

  if [ -z "$version" ]; then
    error "Failed to fetch latest version from GitHub"
  fi
  echo "$version"
}

download_and_install() {
  local platform="$1"
  local version="$2"
  local url="https://github.com/${REPO}/releases/download/${version}/${BINARY_NAME}-${platform}.tar.gz"
  local tmpdir

  info "Downloading ${BINARY_NAME} ${version} for ${platform}..."
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' EXIT

  if ! curl -sSfL "$url" -o "$tmpdir/archive.tar.gz"; then
    error "Download failed. Check that version ${version} exists for platform ${platform}"
  fi

  tar xzf "$tmpdir/archive.tar.gz" -C "$tmpdir"

  # Find the binary in the extracted archive
  local bin_path
  bin_path="$(find "$tmpdir" -name "$BINARY_NAME" -type f | head -1)"
  if [ -z "$bin_path" ]; then
    error "Binary not found in archive"
  fi

  mkdir -p "$INSTALL_DIR"
  install -m 755 "$bin_path" "$INSTALL_DIR/$BINARY_NAME"
  info "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
}

create_config() {
  if [ -f "$CONFIG_FILE" ]; then
    info "Config already exists: ${CONFIG_FILE} (skipping)"
    return
  fi

  mkdir -p "$CONFIG_DIR"
  cat > "$CONFIG_FILE" <<'TOML'
[client]
# Server hostname or URL
# Option 1: Full URL (recommended)
#   server_host = "https://tunnel.example.com:443"
# Option 2: Separate host + port
server_host = "tunnel.example.com"
server_port = 8443

# Keys - use env vars or paste directly
# Example: private_key = "${CLIENT_PRIVATE_KEY}"
private_key       = ""  # Client private key
server_public_key = ""  # Server public key

[local]
host = "127.0.0.1"
port = 3000  # Port of your local service

[reconnect]
enabled = true
initial_delay_ms = 1000
max_delay_ms = 30000

[dashboard]
enabled = true
port = 3000
max_events = 500
TOML
  info "Created config: ${CONFIG_FILE}"
  warn "Edit ${CONFIG_FILE} with your keys and server address"
}

install_systemd_service() {
  if [ "$(uname -s)" != "Linux" ]; then
    info "Skipping systemd setup (not Linux)"
    return
  fi

  if ! command -v systemctl &>/dev/null; then
    info "Skipping systemd setup (systemctl not found)"
    return
  fi

  mkdir -p "$SERVICE_DIR"
  cat > "$SERVICE_DIR/shitty-tunnel-client.service" <<EOF
[Unit]
Description=shittyTunnel Client
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/${BINARY_NAME} client --config ${CONFIG_FILE}
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

  systemctl --user daemon-reload
  info "Installed systemd user service: shitty-tunnel-client"
}

update_path() {
  if echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    return
  fi

  warn "${INSTALL_DIR} is not in your PATH"
  echo ""
  echo "  Add it to your shell profile:"
  echo "    export PATH=\"${INSTALL_DIR}:\$PATH\""
  echo ""
}

main() {
  echo ""
  echo "  ╔═══════════════════════════════════════╗"
  echo "  ║   shittyTunnel Client Installer       ║"
  echo "  ╚═══════════════════════════════════════╝"
  echo ""

  local platform version
  platform="$(detect_platform)"
  version="$(get_latest_version)"

  download_and_install "$platform" "$version"
  create_config
  install_systemd_service
  update_path

  echo ""
  echo "  Installation complete!"
  echo ""
  echo "  Next steps:"
  echo "    1. Edit ~/.config/shittyTunnel.toml with your keys and server"
  echo "    2. Generate keys: ${INSTALL_DIR}/${BINARY_NAME} keygen"
  echo "    3. Start manually: ${INSTALL_DIR}/${BINARY_NAME} client"
  echo "    4. Or enable service: systemctl --user enable --now shitty-tunnel-client"
  echo "    5. Keep running after logout: loginctl enable-linger \$USER"
  echo ""
}

main "$@"
