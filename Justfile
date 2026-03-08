set shell := ["bash", "-euo", "pipefail", "-c"]

default:
    @just --list

# Show all available recipes.
help:
    @just --list

# Build the workspace.
build:
    cargo build

# Run the editor application.
run-editor:
    cargo run -p toki-editor

# Run the runtime application.
run-runtime:
    cargo run -p toki-runtime

# Run all workspace tests.
test:
    cargo test --workspace

# Format all Rust code.
fmt:
    cargo fmt --all

# Check formatting without changing files.
fmt-check:
    cargo fmt --all -- --check

# Run Clippy with warnings treated as errors.
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Install cargo-llvm-cov.
install-llvm-cov:
    cargo install cargo-llvm-cov

# Install cargo-release.
install-cargo-release:
    cargo install cargo-release

# Install cargo-deny.
install-cargo-deny:
    cargo install cargo-deny

# Install cargo-about.
install-cargo-about:
    cargo install cargo-about

# Dry-run a workspace release (no publish, no push).
release-dry-run version:
    cargo release {{version}} --workspace --no-publish

# Execute a workspace release commit/tag locally (no publish, no push).
release-execute version:
    cargo release {{version}} --workspace --no-publish --execute

# Open coverage report for toki-core.
coverage-open:
    cargo llvm-cov -p toki-core --open

# Print workspace coverage summary.
coverage-summary:
    cargo llvm-cov --workspace --summary-only

# Print workspace coverage summary with all features enabled.
coverage:
    cargo llvm-cov --workspace --all-features --summary-only

# Generate workspace rustdoc with Mermaid support (public items only).
quality-docs:
    RUSTDOCFLAGS="--html-in-header docs/mermaid-header.html" cargo doc --locked --workspace --no-deps

# Verify third-party dependency licenses against deny policy.
quality-licenses-check:
    ./scripts/check-licenses.sh

# Generate third-party license inventory from dependency metadata.
quality-licenses-generate:
    ./scripts/generate-third-party-licenses.sh

# Important local checks before pushing.
important: build fmt-check clippy test

# Fast quality gate for LLM-assisted edit loops.
llm: fmt-check clippy test

# Common local CI subset.
ci-local: important
