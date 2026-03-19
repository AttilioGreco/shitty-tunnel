#!/usr/bin/env bash
set -euo pipefail

# shittyTunnel Server Installer
# Usage: curl -sSL https://raw.githubusercontent.com/AttilioGreco/shitty-tunnel/main/scripts/install-server.sh | bash
#
# Environment variables:
#   SHITTY_TUNNEL_VERSION   - specific version to install (default: latest)
#   SHITTY_TUNNEL_INSTALL_DIR - install directory (default: /usr/local/bin)

REPO="AttilioGreco/shitty-tunnel"
BINARY_NAME="shitty-tunnel"
INSTALL_DIR="${SHITTY_TUNNEL_INSTALL_DIR:-/usr/local/bin}"
CONFIG_DIR="/etc/shittyTunnel"
CONFIG_FILE="${CONFIG_DIR}/server.toml"

info()  { echo "  [+] $*"; }
warn()  { echo "  [!] $*" >&2; }
error() { echo "  [✗] $*" >&2; exit 1; }

require_root() {
  if [ "$(id -u)" -ne 0 ]; then
    error "Server installation requires root. Run with: curl ... | sudo bash"
  fi
}

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

  install -m 755 "$bin_path" "$INSTALL_DIR/$BINARY_NAME"
  info "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
}

create_config() {
  if [ -f "$CONFIG_FILE" ]; then
    info "Config already exists: ${CONFIG_FILE} (skipping)"
    return
  fi

  mkdir -p "$CONFIG_DIR"
  chmod 750 "$CONFIG_DIR"

  cat > "$CONFIG_FILE" <<'TOML'
[server]
public_port = 8080    # Public HTTP port (receives from nginx/ingress)
tunnel_port = 8443    # gRPC port for tunnel clients

# Server private key - generate with: shitty-tunnel keygen
# You can use environment variables: private_key = "${SERVER_PRIVATE_KEY}"
private_key = ""

# Optional: Native TLS on tunnel_port (if not using ingress)
# [server.tls]
# enabled = false
# cert_path = "/path/to/cert.pem"
# key_path = "/path/to/key.pem"

# Authorized clients (one per developer)
# [[peers]]
# public_key = "client_public_key_here"
# domain = "dev1.example.com"
TOML

  chmod 640 "$CONFIG_FILE"
  info "Created config: ${CONFIG_FILE}"
  warn "Edit ${CONFIG_FILE} with your keys and peers"
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

  cat > /etc/systemd/system/shitty-tunnel-server.service <<EOF
[Unit]
Description=shittyTunnel Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${INSTALL_DIR}/${BINARY_NAME} server --config ${CONFIG_FILE}
Restart=on-failure
RestartSec=5
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadOnlyPaths=${CONFIG_DIR}

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  info "Installed systemd service: shitty-tunnel-server"
}

main() {
  echo ""
  echo "  ╔═══════════════════════════════════════╗"
  echo "  ║   shittyTunnel Server Installer       ║"
  echo "  ╚═══════════════════════════════════════╝"
  echo ""

  require_root

  local platform version
  platform="$(detect_platform)"
  version="$(get_latest_version)"

  download_and_install "$platform" "$version"
  create_config
  install_systemd_service

  echo ""
  echo "  Installation complete!"
  echo ""
  echo "  Next steps:"
  echo "    1. Generate keys: ${INSTALL_DIR}/${BINARY_NAME} keygen"
  echo "    2. Edit ${CONFIG_FILE} with your keys and peers"
  echo "    3. Start: systemctl enable --now shitty-tunnel-server"
  echo "    4. Logs:  journalctl -u shitty-tunnel-server -f"
  echo ""
}

main "$@"
