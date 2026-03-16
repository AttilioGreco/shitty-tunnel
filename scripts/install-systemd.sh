#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  install-systemd.sh --mode server [--bin PATH] [--config PATH]
  install-systemd.sh --mode client [--bin PATH] [--config PATH]

Options:
  --mode    server | client
  --bin     full path to shitty-tunnel binary
            default: $HOME/.local/bin/shitty-tunnel
  --config  config file path
            server default: /etc/shittyTunnel/server.toml
            client default: $HOME/.config/shittyTunnel.toml
EOF
}

MODE=""
BIN_PATH="${HOME}/.local/bin/shitty-tunnel"
CONFIG_PATH=""

while [ $# -gt 0 ]; do
  case "$1" in
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    --bin)
      BIN_PATH="${2:-}"
      shift 2
      ;;
    --config)
      CONFIG_PATH="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [ -z "$MODE" ]; then
  echo "--mode is required" >&2
  usage
  exit 1
fi

if [ ! -x "$BIN_PATH" ]; then
  echo "Binary not found or not executable: $BIN_PATH" >&2
  exit 1
fi

if [ "$MODE" = "server" ]; then
  if [ -z "$CONFIG_PATH" ]; then
    CONFIG_PATH="/etc/shittyTunnel/server.toml"
  fi

  sudo mkdir -p /etc/systemd/system /etc/shittyTunnel
  sudo tee /etc/systemd/system/shitty-tunnel-server.service > /dev/null <<EOF
[Unit]
Description=shittyTunnel Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${BIN_PATH} server --config ${CONFIG_PATH}
Restart=on-failure
RestartSec=5
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=read-only
ReadOnlyPaths=/etc/shittyTunnel

[Install]
WantedBy=multi-user.target
EOF

  sudo systemctl daemon-reload
  echo "Installed /etc/systemd/system/shitty-tunnel-server.service"
  echo "Enable now: sudo systemctl enable --now shitty-tunnel-server"
  exit 0
fi

if [ "$MODE" = "client" ]; then
  if [ -z "$CONFIG_PATH" ]; then
    CONFIG_PATH="${HOME}/.config/shittyTunnel.toml"
  fi

  mkdir -p "${HOME}/.config/systemd/user"
  tee "${HOME}/.config/systemd/user/shitty-tunnel-client.service" > /dev/null <<EOF
[Unit]
Description=shittyTunnel Client
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=${BIN_PATH} client --config ${CONFIG_PATH}
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

  systemctl --user daemon-reload
  echo "Installed ${HOME}/.config/systemd/user/shitty-tunnel-client.service"
  echo "Enable now: systemctl --user enable --now shitty-tunnel-client"
  exit 0
fi

echo "Invalid --mode: $MODE (expected server|client)" >&2
exit 1
