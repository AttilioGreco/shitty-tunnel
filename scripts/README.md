# Release Scripts

## release.sh

Automated release script that handles versioning, changelog updates, git tagging, and publishing.

### Usage

```bash
./scripts/release.sh <version> [options]
```

### Examples

```bash
# Standard release
./scripts/release.sh 0.1.0

# Release without running tests
./scripts/release.sh 0.1.0 --skip-tests

# Prepare release but don't push (dry-run)
./scripts/release.sh 0.1.0 --no-push

# Pre-release version
./scripts/release.sh 1.0.0-beta.1
```

### What it does

1. **Validates** version format (semver)
2. **Checks** for uncommitted changes
3. **Runs** tests (`cargo test --workspace`)
4. **Updates** `Cargo.toml` with new version
5. **Updates** `CHANGELOG.md` (moves Unreleased → version)
6. **Commits** changes with `chore: bump version to vX.Y.Z`
7. **Creates** git tag `vX.Y.Z`
8. **Tests** Docker build (optional check)
9. **Pushes** commit and tag to remote

### Options

- `--skip-tests` - Skip running `cargo test`
- `--no-push` - Prepare release locally but don't push to remote

### What happens after push

When you push the tag, two GitHub Actions workflows will trigger:

1. **Release** (`release.yml`) - powered by cargo-dist
   - Builds binaries for Linux, macOS, Windows
   - Creates GitHub Release
   - Uploads installers

2. **Docker Publish** (`docker-publish.yml`)
   - Builds multi-arch images (amd64, arm64)
   - Pushes to ghcr.io
   - Runs Trivy security scan
   - Uploads results to GitHub Security tab

### Troubleshooting

**"You have uncommitted changes"**
```bash
git status
git add .
git commit -m "your changes"
# Then run release script again
```

**"Tests failed"**
```bash
cargo test --workspace  # See which tests are failing
# Fix tests, or use --skip-tests to bypass (not recommended)
```

**Want to undo a release (before push)?**
```bash
# Delete the tag
git tag -d v0.1.0

# Reset the commit
git reset --hard HEAD~1

# Restore original files (if needed)
git checkout Cargo.toml CHANGELOG.md
```

**Already pushed but want to cancel?**
- Can't easily undo - the workflows will have triggered
- You can delete the tag on GitHub and the release
- But better to fix forward with a new patch version

### Version Guidelines

- **Patch** (0.0.X): Bug fixes, small improvements
- **Minor** (0.X.0): New features, backwards compatible
- **Major** (X.0.0): Breaking changes
- **Pre-release** (1.0.0-beta.1): Testing before stable

### See Also

- [Release Checklist](../.github/RELEASE_CHECKLIST.md)
- [Docker Documentation](../.github/DOCKER.md)
