# Release Process

This project uses [cargo-dist](https://github.com/axodotdev/cargo-dist) for automated multi-platform releases.

## Automated Release (GitHub Actions)

### 1. Update version

```bash
# Update version in Cargo.toml (workspace.package.version)
vim Cargo.toml

# Commit
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 0.2.0"
```

### 2. Update CHANGELOG.md

```bash
vim CHANGELOG.md
# Move [Unreleased] items to [0.2.0] section
# Add release date

git add CHANGELOG.md
git commit -m "docs: update changelog for v0.2.0"
```

### 3. Create and push tag

```bash
# Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0"

# Push tag to GitHub
git push origin v0.2.0
```

### 4. Wait for CI

GitHub Actions will automatically:
- Build binaries for all platforms (Linux x64/ARM, macOS Intel/ARM, Windows)
- Generate installers (shell script, PowerShell, Homebrew)
- Create checksums and signatures
- Create GitHub Release with artifacts
- Generate release notes from CHANGELOG.md

**Build targets:**
- `x86_64-unknown-linux-gnu` - Linux x64 (glibc)
- `x86_64-unknown-linux-musl` - Linux x64 (musl static)
- `aarch64-unknown-linux-gnu` - Linux ARM64 (glibc)
- `aarch64-unknown-linux-musl` - Linux ARM64 (musl static)
- `x86_64-apple-darwin` - macOS Intel
- `aarch64-apple-darwin` - macOS Apple Silicon
- `x86_64-pc-windows-msvc` - Windows x64

### 5. Verify release

Check GitHub Releases page: `https://github.com/yourusername/shittyTunnel/releases`

Test installers:
```bash
# Shell (Linux/macOS)
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/yourusername/shittyTunnel/releases/download/v0.2.0/shitty-tunnel-installer.sh | sh

# Homebrew
brew install yourusername/tap/shitty-tunnel

# PowerShell (Windows)
irm https://github.com/yourusername/shittyTunnel/releases/download/v0.2.0/shitty-tunnel-installer.ps1 | iex
```

---

## Manual Release (Local build)

For testing or custom builds:

```bash
# Build all platforms
./build-release.sh 0.2.0

# Or build natively
./build-native.sh 0.2.0

# Artifacts in: build/
```

---

## Troubleshooting

### CI fails on specific platform

Check `.github/workflows/release.yml` for platform-specific issues.

Common fixes:
- Update cargo-dist version in `dist-workspace.toml`
- Check for platform-specific dependencies
- Verify protoc installation in CI

### Installer not working

cargo-dist installers require:
- GitHub Release must be published (not draft)
- Artifacts must be uploaded
- Release must have tag matching semver pattern

### Update cargo-dist

```bash
cargo install cargo-dist@latest --locked
dist generate
git add .github/ dist-workspace.toml
git commit -m "chore: update cargo-dist"
```

---

## Version Scheme

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (1.0.0): Breaking API changes
- **MINOR** (0.1.0): New features, backward compatible
- **PATCH** (0.1.1): Bug fixes, backward compatible

Examples:
- `v0.1.0` → First release
- `v0.1.1` → Bug fix
- `v0.2.0` → New feature (reconnect improvements)
- `v1.0.0` → Stable API

---

## Pre-releases

For beta/RC versions:

```bash
git tag -a v0.2.0-beta.1 -m "Release v0.2.0-beta.1"
git push origin v0.2.0-beta.1
```

GitHub Release will be marked as "Pre-release".
