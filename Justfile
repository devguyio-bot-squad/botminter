# botminter Justfile
# Development and documentation tasks for the bm CLI.

# Generator root (where this Justfile lives)
generator_root := justfile_directory()

# Default recipe — list available commands
default:
    @just --list

# Build the bm CLI binary
build:
    cargo build -p bm

# Run all tests
test:
    cargo test -p bm

# Run E2E tests (requires gh auth + podman)
e2e:
    cargo test -p bm --features e2e -- --test-threads=1

# Run E2E tests with output visible
e2e-verbose:
    cargo test -p bm --features e2e -- --test-threads=1 --nocapture

# Run clippy with warnings as errors
clippy:
    cargo clippy -p bm -- -D warnings

# Set up docs virtual environment and install dependencies (idempotent)
docs-setup:
    #!/usr/bin/env bash
    set -euo pipefail
    DOCS_DIR="{{ generator_root }}/docs"
    VENV_DIR="$DOCS_DIR/.venv"
    if [ ! -f "$VENV_DIR/bin/zensical" ]; then
        echo "Setting up docs virtualenv..."
        python3 -m venv "$VENV_DIR"
        "$VENV_DIR/bin/pip" install --quiet -r "$DOCS_DIR/requirements.txt"
        echo "Docs dependencies installed."
    else
        echo "Docs virtualenv already set up."
    fi

# Start live-reload dev server at localhost:8000
docs-serve: docs-setup
    #!/usr/bin/env bash
    set -euo pipefail
    DOCS_DIR="{{ generator_root }}/docs"
    "$DOCS_DIR/.venv/bin/zensical" serve -f "$DOCS_DIR/mkdocs.yml"

# Build static docs site to docs/site/
docs-build: docs-setup
    #!/usr/bin/env bash
    set -euo pipefail
    DOCS_DIR="{{ generator_root }}/docs"
    "$DOCS_DIR/.venv/bin/zensical" build -f "$DOCS_DIR/mkdocs.yml"
    echo "Site built at $DOCS_DIR/site/"

# Tag, create GitHub release with notes, and push
release version notes_file:
    #!/usr/bin/env bash
    set -euo pipefail
    VERSION="{{ version }}"
    NOTES_FILE="{{ notes_file }}"
    if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+'; then
        echo "Error: version must be semver (e.g., 0.2.0)" >&2
        exit 1
    fi
    if [ ! -f "$NOTES_FILE" ]; then
        echo "Error: notes file '$NOTES_FILE' not found" >&2
        exit 1
    fi
    TAG="v${VERSION}"
    # Update version in Cargo.toml
    sed -i 's/^version = ".*"/version = "'"$VERSION"'"/' crates/bm/Cargo.toml
    # Update Cargo.lock
    cargo generate-lockfile
    # Commit version bump if there are changes
    git add crates/bm/Cargo.toml Cargo.lock
    if ! git diff --cached --quiet; then
        git commit -s -S -m "chore(release): bump version to ${VERSION}"
    fi
    git tag -s -a "$TAG" -m "Release ${TAG}"
    git push origin HEAD "$TAG"
    # Create GitHub release with notes from file
    REPO=$(git remote get-url origin | sed -E 's|.*github\.com[:/](.+)\.git$|\1|')
    gh release create "$TAG" --repo "$REPO" --title "$TAG" --notes-file "$NOTES_FILE"
    echo "Released ${TAG} — workflow will build and attach binaries"

# Build locally and attach binary to an existing release (fallback if CI fails)
release-build-local tag:
    #!/usr/bin/env bash
    set -euo pipefail
    TAG="{{ tag }}"
    REPO=$(git remote get-url origin | sed -E 's|.*github\.com[:/](.+)\.git$|\1|')
    # Verify the release exists
    if ! gh release view "$TAG" --repo "$REPO" > /dev/null 2>&1; then
        echo "Error: release '$TAG' not found" >&2
        exit 1
    fi
    TARGET=$(rustc -vV | grep '^host:' | awk '{print $2}')
    echo "Building for ${TARGET}..."
    cargo build --release -p bm --target "$TARGET"
    cd "target/${TARGET}/release"
    tar czf "../../../bm-${TARGET}.tar.gz" bm
    cd ../../..
    gh release upload "$TAG" "bm-${TARGET}.tar.gz" --repo "$REPO"
    rm "bm-${TARGET}.tar.gz"
    echo "Attached bm-${TARGET}.tar.gz to ${TAG}"
