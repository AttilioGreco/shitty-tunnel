# Release Checklist

Quick reference for releasing a new version.

## Pre-release

- [ ] All tests pass: `cargo test --workspace`
- [ ] Code builds on all platforms: `dist plan`
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
- [ ] Test installer scripts
- [ ] Update README if needed
- [ ] Announce on relevant channels

## Installers Generated

After release, users can install via:

```bash
# Shell script (Linux/macOS)
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/yourusername/shittyTunnel/releases/latest/download/shitty-tunnel-installer.sh | sh

# Homebrew
brew install yourusername/tap/shitty-tunnel

# PowerShell (Windows)
irm https://github.com/yourusername/shittyTunnel/releases/latest/download/shitty-tunnel-installer.ps1 | iex

# Direct download
# https://github.com/yourusername/shittyTunnel/releases
```

## Platform Matrix

| Platform | Target | Static | Notes |
|----------|--------|--------|-------|
| Linux x64 (glibc) | x86_64-unknown-linux-gnu | ❌ | Debian/Ubuntu/Fedora |
| Linux x64 (musl) | x86_64-unknown-linux-musl | ✅ | Universal Linux |
| Linux ARM64 (glibc) | aarch64-unknown-linux-gnu | ❌ | Pi 4/5, ARM servers |
| Linux ARM64 (musl) | aarch64-unknown-linux-musl | ✅ | Universal ARM |
| macOS Intel | x86_64-apple-darwin | ❌ | Intel Mac |
| macOS ARM | aarch64-apple-darwin | ❌ | M1/M2/M3 |
| Windows | x86_64-pc-windows-msvc | ❌ | Windows 10+ |
