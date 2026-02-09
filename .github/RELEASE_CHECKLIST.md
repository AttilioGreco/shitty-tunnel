# Release Checklist

Quick reference for releasing a new version.

## Pre-release

- [ ] All tests pass: `cargo test --workspace`
- [ ] Code builds on all platforms: `dist plan`
- [ ] Docker image builds locally: `docker build -t shittytunnel:test .`
- [ ] Update CHANGELOG.md with new version
- [ ] Update version in `Cargo.toml` (workspace.package.version)
- [ ] Commit changes: `git commit -am "chore: bump version to vX.Y.Z"`

## Release

```bash
# Create and push tag
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin vX.Y.Z
```

## Post-release

- [ ] Verify GitHub Release created
- [ ] Verify Docker images published on ghcr.io
- [ ] Test Docker image: `docker run ghcr.io/agreco/shittytunnel:vX.Y.Z --version`
- [ ] Test installer scripts
- [ ] Check Trivy security scan results in Security tab
- [ ] Update README if needed
- [ ] Announce on relevant channels

## Installation Methods

After release, users can install via:

### Docker (Recommended)

```bash
# Pull latest
docker pull ghcr.io/agreco/shittytunnel:latest

# Pull specific version
docker pull ghcr.io/agreco/shittytunnel:vX.Y.Z

# Run
docker run ghcr.io/agreco/shittytunnel:latest --help
```

### Binary Installers

```bash
# Shell script (Linux/macOS)
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/agreco/shittyTunnel/releases/latest/download/shitty-tunnel-installer.sh | sh

# Homebrew
brew install agreco/tap/shitty-tunnel

# PowerShell (Windows)
irm https://github.com/agreco/shittyTunnel/releases/latest/download/shitty-tunnel-installer.ps1 | iex

# Direct download
# https://github.com/agreco/shittyTunnel/releases
```
