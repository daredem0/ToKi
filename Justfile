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

# Open coverage report for toki-core.
coverage-open:
    cargo llvm-cov -p toki-core --open

# Print workspace coverage summary.
coverage-summary:
    cargo llvm-cov --workspace --summary-only

# Print workspace coverage summary with all features enabled.
coverage:
    cargo llvm-cov --workspace --all-features --summary-only

# Important local checks before pushing.
important: build fmt-check clippy test

# Fast quality gate for LLM-assisted edit loops.
llm: fmt-check clippy test

# Common local CI subset.
ci-local: important
