#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

if ! cargo about --version >/dev/null 2>&1; then
  echo "cargo-about is not installed." >&2
  echo "Install with: cargo install --locked cargo-about" >&2
  exit 1
fi

# cargo-about may emit non-fatal parser diagnostics for deprecated SPDX IDs
# contained in upstream source trees. Keep output clean by default.
LOG_LEVEL="${CARGO_ABOUT_LOG_LEVEL:-off}"

cargo about -L "${LOG_LEVEL}" generate --locked about.hbs > THIRD_PARTY_LICENSES.md
echo "Wrote THIRD_PARTY_LICENSES.md"
