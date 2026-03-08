#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

if ! TAG="$(git describe --tags --exact-match HEAD 2>/dev/null)"; then
  echo "No tag on HEAD; skipping tag/version consistency check."
  exit 0
fi

VERSION_IN_TAG="${TAG#v}"

WORKSPACE_VERSION="$(
  awk '
    /^\[workspace\.package\]/ { in_section = 1; next }
    /^\[/ { in_section = 0 }
    in_section && /^[[:space:]]*version[[:space:]]*=/ {
      line = $0
      sub(/^[^=]*=[[:space:]]*"/, "", line)
      sub(/".*$/, "", line)
      print line
      exit
    }
  ' Cargo.toml
)"

if [[ -z "${WORKSPACE_VERSION}" ]]; then
  echo "Unable to read workspace.package.version from Cargo.toml" >&2
  exit 1
fi

if [[ "${VERSION_IN_TAG}" != "${WORKSPACE_VERSION}" ]]; then
  echo "Tag/version mismatch: tag=${VERSION_IN_TAG}, workspace.version=${WORKSPACE_VERSION}" >&2
  exit 1
fi

echo "Tag/version consistency OK: ${VERSION_IN_TAG}"
