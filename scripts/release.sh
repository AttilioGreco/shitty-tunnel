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

# Every confirmation below treats an empty answer as "yes", which is a sensible
# default at a terminal but means a piped or redirected run auto-approves the
# commit, the tag and the push without anyone agreeing to any of them.
if [ ! -t 0 ]; then
    error "Run this from a terminal: the confirmation prompts read as 'yes' on empty input."
fi

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

# Preconditions.
#
# Everything below this block edits files, runs the test suite and builds two
# images before it ever reaches `git tag`. A release that cannot succeed must
# be rejected here, while the tree is still untouched — otherwise a rejected
# run leaves a bump commit, a rewritten CHANGELOG and a rewound version behind,
# and the next attempt starts from that mess.
CURRENT_VERSION=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)

if [ -z "$CURRENT_VERSION" ]; then
    error "Could not read the current version from Cargo.toml"
fi

if git rev-parse -q --verify "refs/tags/${TAG}" > /dev/null; then
    error "Tag ${TAG} already exists locally. Releases are immutable — pick a higher version."
fi

if git ls-remote --exit-code --tags origin "refs/tags/${TAG}" > /dev/null 2>&1; then
    error "Tag ${TAG} already exists on origin. Releases are immutable — pick a higher version."
fi

# Compare against the highest tag as well as Cargo.toml, not just Cargo.toml.
# A failed release can leave the manifest rewound to an old version, and then
# any number above that rewound value looks new while actually being a
# re-release — the tag check only catches it if that exact tag exists.
HIGHEST_TAG=$(git tag --list 'v*' | sed 's/^v//' | sort -V | tail -1)
FLOOR="$CURRENT_VERSION"
if [ -n "$HIGHEST_TAG" ] && \
   [ "$(printf '%s\n%s\n' "$CURRENT_VERSION" "$HIGHEST_TAG" | sort -V | tail -1)" = "$HIGHEST_TAG" ]; then
    FLOOR="$HIGHEST_TAG"
fi

# `sort -V` orders release versions correctly; it does not implement semver
# prerelease precedence, so X.Y.Z-rc1 sorts after X.Y.Z. Good enough to catch
# the failure that actually happens: re-releasing an old number.
if [ "$VERSION" = "$FLOOR" ] || \
   [ "$(printf '%s\n%s\n' "$FLOOR" "$VERSION" | sort -V | tail -1)" != "$VERSION" ]; then
    if [ "$FLOOR" = "$CURRENT_VERSION" ]; then
        error "Version ${VERSION} is not newer than the current ${CURRENT_VERSION}."
    fi
    error "Version ${VERSION} is not newer than the highest released ${FLOOR} (Cargo.toml says ${CURRENT_VERSION} — a previous release probably failed midway)."
fi

if grep -q "^## \[${VERSION}\]" CHANGELOG.md; then
    error "CHANGELOG.md already has an entry for ${VERSION}."
fi

success "Preconditions passed (current version: ${CURRENT_VERSION})"

# Undo the bump commit if the release fails between committing and tagging.
# Outside that window there is nothing to undo: before it no commit exists,
# after it the tag refers to the commit and removing it would orphan the tag.
BUMP_COMMIT=""
TAGGED=false
rollback_bump_commit() {
    [ -n "$BUMP_COMMIT" ] || return 0
    [ "$TAGGED" = false ] || return 0
    [ "$(git rev-parse HEAD)" = "$BUMP_COMMIT" ] || return 0
    warn "Release failed after committing — removing the bump commit"
    git reset --hard HEAD~1 > /dev/null
}
trap rollback_bump_commit EXIT

# Helper to revert file changes on failure
revert_files() {
    warn "Reverting file changes..."
    git checkout Cargo.toml Cargo.lock CHANGELOG.md dist-workspace.toml .github/workflows/release.yml 2>/dev/null || true
}

# 1. Update Cargo.toml version
info "Updating Cargo.toml version to ${VERSION}..."
CARGO_FILE="Cargo.toml"

cp "$CARGO_FILE" "${CARGO_FILE}.bak"

if sed -i.tmp "s/^version = \".*\"$/version = \"${VERSION}\"/" "$CARGO_FILE" && \
   grep -q "version = \"${VERSION}\"" "$CARGO_FILE"; then
    rm -f "${CARGO_FILE}.tmp" "${CARGO_FILE}.bak"
    success "Updated Cargo.toml"
else
    mv "${CARGO_FILE}.bak" "$CARGO_FILE"
    error "Failed to update Cargo.toml"
fi

# 2. Update CHANGELOG.md
info "Updating CHANGELOG.md..."
CHANGELOG="CHANGELOG.md"
TODAY=$(date +%Y-%m-%d)

cp "$CHANGELOG" "${CHANGELOG}.bak"

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

# 3. Run tests (with updated Cargo.toml so the tested artifact matches the release)
if [ "$SKIP_TESTS" = false ]; then
    info "Running tests..."
    if cargo test --workspace --quiet; then
        success "All tests passed"
    else
        revert_files
        error "Tests failed. Fix them or use --skip-tests to skip."
    fi
else
    warn "Skipping tests (--skip-tests flag)"
fi
echo

# 4. Build release binary with fresh embedded frontend assets
info "Building release binary with fresh frontend assets..."
rm -rf frontend/dist
if ST_REQUIRE_FRONTEND=1 cargo build --release --bin shitty-tunnel --quiet; then
    success "Release build successful (frontend rebuilt)"
else
    revert_files
    error "Release build failed (including frontend embed build)"
fi
echo

# 5. Refresh cargo-dist generated files (release workflow)
info "Refreshing cargo-dist generated files..."
if ! cargo dist --version > /dev/null 2>&1; then
    revert_files
    error "cargo-dist is required for release checks. Install with: cargo install cargo-dist --locked --version 0.30.3"
fi

if cargo dist generate-ci > /dev/null 2>&1; then
    success "cargo-dist workflow regenerated"
else
    revert_files
    error "Failed to regenerate cargo-dist workflow"
fi
echo

# 6. Show changes
echo
info "Changes to be committed:"
git diff --color=always Cargo.toml Cargo.lock CHANGELOG.md dist-workspace.toml .github/workflows/release.yml | head -80
echo

# 7. Confirm before committing
read -p "$(echo -e ${GREEN}Commit these changes?${NC} [Y/n] )" -n 1 -r
echo
if [[ $REPLY =~ ^[Nn]$ ]]; then
    warn "Restoring original files..."
    git checkout Cargo.toml Cargo.lock CHANGELOG.md dist-workspace.toml .github/workflows/release.yml
    error "Release cancelled"
fi

# 8. Commit changes
info "Creating commit..."
git add Cargo.toml Cargo.lock CHANGELOG.md dist-workspace.toml .github/workflows/release.yml
if git commit -m "chore: bump version to ${TAG}"; then
    BUMP_COMMIT=$(git rev-parse HEAD)
    success "Commit created"
else
    error "Failed to create commit"
fi

# 9. cargo-dist preflight (same dist planning step used in CI)
echo
info "Running cargo-dist preflight..."
DIST_PLAN_FILE="/tmp/plan-dist-manifest-${TAG}.json"
if dist host --steps=create --tag="${TAG}" --output-format=json > "${DIST_PLAN_FILE}"; then
    success "cargo-dist preflight passed"
else
    error "cargo-dist preflight failed. Fix the reported issue before creating the tag."
fi

# 10. Build check (optional) — before tagging so a failure leaves no tag behind
echo
info "Testing Docker build..."
if docker build -t "shittytunnel:${VERSION}" . > /dev/null 2>&1; then
    success "Docker build successful"
else
    warn "Docker build failed (this won't prevent the release)"
fi

# 11. Create git tag — last git operation before push
info "Creating tag ${TAG}..."
if git tag -a "$TAG" -m "Release ${TAG}"; then
    TAGGED=true
    success "Tag created: ${TAG}"
else
    error "Failed to create tag"
fi

# 12. Push to remote
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
            echo "  1. Monitor GitHub Actions: https://github.com/AttilioGreco/shitty-tunnel/actions"
            echo "  2. Check release page: https://github.com/AttilioGreco/shitty-tunnel/releases/tag/${TAG}"
            echo "  3. Verify Docker image: docker pull ghcr.io/AttilioGreco/shitty-tunnel:${VERSION}"
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
