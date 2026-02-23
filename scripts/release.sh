#!/usr/bin/env bash
set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() { echo -e "${BLUE}[i]${NC} $*"; }
success() { echo -e "${GREEN}[OK]${NC} $*"; }
warn() { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# Check if version is provided
if [ $# -eq 0 ]; then
    error "Usage: $0 <version> [--skip-tests] [--no-push]"
fi

VERSION="$1"
SKIP_TESTS=false
NO_PUSH=false

# Parse additional flags
shift
while [ $# -gt 0 ]; do
    case "$1" in
        --skip-tests) SKIP_TESTS=true ;;
        --no-push) NO_PUSH=true ;;
        *) warn "Unknown flag: $1" ;;
    esac
    shift
done

# Validate version format (semver)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    error "Invalid version format. Expected: X.Y.Z or X.Y.Z-prerelease"
fi

# Remove 'v' prefix if present
VERSION="${VERSION#v}"
TAG="v${VERSION}"

info "Preparing release ${YELLOW}${TAG}${NC}"
echo

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    error "You have uncommitted changes. Please commit or stash them first."
fi

# Check if we're on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "main" ]; then
    warn "You're not on 'main' branch (current: ${CURRENT_BRANCH})"
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        error "Release cancelled"
    fi
fi

# 1. Run tests (unless skipped)
if [ "$SKIP_TESTS" = false ]; then
    info "Running tests..."
    if cargo test --workspace --quiet; then
        success "All tests passed"
    else
        error "Tests failed. Fix them or use --skip-tests to skip."
    fi
else
    warn "Skipping tests (--skip-tests flag)"
fi
echo

# 2. Update Cargo.toml version
info "Updating Cargo.toml version to ${VERSION}..."
CARGO_FILE="Cargo.toml"

# Backup original file
cp "$CARGO_FILE" "${CARGO_FILE}.bak"

# Update version in [workspace.package]
if sed -i.tmp "s/^version = \".*\"$/version = \"${VERSION}\"/" "$CARGO_FILE" && \
   grep -q "version = \"${VERSION}\"" "$CARGO_FILE"; then
    rm -f "${CARGO_FILE}.tmp" "${CARGO_FILE}.bak"
    success "Updated Cargo.toml"
else
    mv "${CARGO_FILE}.bak" "$CARGO_FILE"
    error "Failed to update Cargo.toml"
fi

# 3. Update CHANGELOG.md
info "Updating CHANGELOG.md..."
CHANGELOG="CHANGELOG.md"
TODAY=$(date +%Y-%m-%d)

# Backup original file
cp "$CHANGELOG" "${CHANGELOG}.bak"

# Replace [Unreleased] with [version] - date and create new [Unreleased] section
if awk -v ver="$VERSION" -v date="$TODAY" '
    /^## \[Unreleased\]/ {
        print "## [Unreleased]\n"
        print "## [" ver "] - " date
        next
    }
    { print }
' "$CHANGELOG" > "${CHANGELOG}.tmp" && mv "${CHANGELOG}.tmp" "$CHANGELOG"; then
    rm -f "${CHANGELOG}.bak"
    success "Updated CHANGELOG.md"
else
    mv "${CHANGELOG}.bak" "$CHANGELOG"
    error "Failed to update CHANGELOG.md"
fi

# 4. Show changes
echo
info "Changes to be committed:"
git diff --color=always Cargo.toml CHANGELOG.md | head -50
echo

# 5. Confirm before committing
read -p "$(echo -e ${GREEN}Commit these changes?${NC} [Y/n] )" -n 1 -r
echo
if [[ $REPLY =~ ^[Nn]$ ]]; then
    warn "Restoring original files..."
    git checkout Cargo.toml CHANGELOG.md
    error "Release cancelled"
fi

# 6. Commit changes
info "Creating commit..."
git add Cargo.toml CHANGELOG.md
if git commit -m "chore: bump version to ${TAG}"; then
    success "Commit created"
else
    error "Failed to create commit"
fi

# 7. Build check (optional) — before tagging so a failure leaves no tag behind
echo
info "Testing Docker build..."
if docker build -t "shittytunnel:${VERSION}" . > /dev/null 2>&1; then
    success "Docker build successful"
else
    warn "Docker build failed (this won't prevent the release)"
fi

# 8. Create git tag — last git operation before push
info "Creating tag ${TAG}..."
if git tag -a "$TAG" -m "Release ${TAG}"; then
    success "Tag created: ${TAG}"
else
    error "Failed to create tag"
fi

# 9. Push to remote
echo
if [ "$NO_PUSH" = true ]; then
    warn "Skipping push (--no-push flag)"
    echo
    info "To push manually, run:"
    echo "  git push origin main"
    echo "  git push origin ${TAG}"
else
    echo -e "${YELLOW}════════════════════════════════════════${NC}"
    echo -e "${YELLOW}Ready to push to remote!${NC}"
    echo -e "${YELLOW}════════════════════════════════════════${NC}"
    echo
    info "This will:"
    echo "  1. Push commit to origin/main"
    echo "  2. Push tag ${TAG}"
    echo "  3. Trigger GitHub Actions:"
    echo "     - Release workflow (binaries + GitHub Release)"
    echo "     - Docker publish workflow (ghcr.io)"
    echo
    read -p "$(echo -e ${GREEN}Push to remote?${NC} [Y/n] )" -n 1 -r
    echo

    if [[ ! $REPLY =~ ^[Nn]$ ]]; then
        info "Pushing to remote..."
        if git push origin "$CURRENT_BRANCH" && git push origin "$TAG"; then
            success "Successfully pushed to remote!"
            echo
            echo -e "${GREEN}════════════════════════════════════════${NC}"
            echo -e "${GREEN}Release ${TAG} is complete!${NC}"
            echo -e "${GREEN}════════════════════════════════════════${NC}"
            echo
            info "Next steps:"
            echo "  1. Monitor GitHub Actions: https://github.com/agreco/shittyTunnel/actions"
            echo "  2. Check release page: https://github.com/agreco/shittyTunnel/releases/tag/${TAG}"
            echo "  3. Verify Docker image: docker pull ghcr.io/agreco/shittytunnel:${VERSION}"
            echo "  4. Check Trivy scan in Security tab"
        else
            error "Failed to push to remote"
        fi
    else
        warn "Push cancelled. To push later, run:"
        echo "  git push origin $CURRENT_BRANCH"
        echo "  git push origin ${TAG}"
    fi
fi

echo
success "Done!"
