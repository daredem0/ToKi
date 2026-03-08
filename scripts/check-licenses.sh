#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

if ! cargo deny --version >/dev/null 2>&1; then
  echo "cargo-deny is not installed." >&2
  echo "Install with: cargo install --locked cargo-deny" >&2
  exit 1
fi

cargo deny check licenses
