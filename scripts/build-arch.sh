#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

resolve_package_suffix() {
  if [[ -n "${TOKI_PACKAGE_SUFFIX:-}" ]]; then
    echo "${TOKI_PACKAGE_SUFFIX}"
    return
  fi

  "${REPO_ROOT}/scripts/detect-libc-suffix.sh"
}

sanitize_package_suffix() {
  local raw="$1"
  local sanitized
  sanitized="$(sed -E 's/[^A-Za-z0-9._+-]+/-/g; s/^-+//; s/-+$//' <<<"${raw}")"
  if [[ -z "${sanitized}" ]]; then
    echo "Derived package suffix is empty after sanitization." >&2
    exit 1
  fi
  echo "${sanitized}"
}

apply_package_suffix() {
  local suffix="$1"
  local files=()
  shopt -s nullglob
  files=(target/arch/*.pkg.tar.*)
  shopt -u nullglob

  if ((${#files[@]} == 0)); then
    echo "No Arch package was produced in target/arch." >&2
    exit 1
  fi

  for pkg in "${files[@]}"; do
    if [[ "${pkg}" == *"-${suffix}.pkg.tar."* ]]; then
      continue
    fi
    local renamed
    renamed="$(sed -E "s/(\\.pkg\\.tar\\.[^.]+)$/-${suffix}\\1/" <<<"${pkg}")"
    if [[ "${renamed}" == "${pkg}" ]]; then
      echo "Unable to apply suffix to package path: ${pkg}" >&2
      exit 1
    fi
    mv "${pkg}" "${renamed}"
  done
}

workspace_version() {
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
}

ensure_prebuilt_inputs() {
  local required=(
    "target/release/toki-editor"
    "target/release/toki-runtime"
    "packaging/arch/PKGBUILD"
    "packaging/toki-editor.desktop"
    "packaging/toki-editor.png"
    "LICENSE-TOKI.md"
    "THIRD_PARTY_LICENSES.md"
  )
  for path in "${required[@]}"; do
    if [[ ! -f "${path}" ]]; then
      echo "Missing required prebuilt artifact: ${path}" >&2
      exit 1
    fi
  done
}

if ! command -v makepkg >/dev/null 2>&1; then
  echo "makepkg is not installed." >&2
  echo "Install with your package manager (on Arch: sudo pacman -S base-devel)." >&2
  exit 1
fi

if [[ "${TOKI_USE_PREBUILT:-0}" == "1" ]]; then
  echo "Using prebuilt release binaries from target/release."
  ensure_prebuilt_inputs
else
  cargo build --locked --release -p toki-runtime -p toki-editor
fi

PKGVER="$(workspace_version)"
if [[ -z "${PKGVER}" ]]; then
  echo "Unable to determine workspace version from Cargo.toml" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "${WORK_DIR}"' EXIT

cp packaging/arch/PKGBUILD "${WORK_DIR}/PKGBUILD"
cp target/release/toki-editor "${WORK_DIR}/toki-editor"
cp target/release/toki-runtime "${WORK_DIR}/toki-runtime"
cp packaging/toki-editor.desktop "${WORK_DIR}/toki-editor.desktop"
cp packaging/toki-editor.png "${WORK_DIR}/toki-editor.png"
cp LICENSE-TOKI.md "${WORK_DIR}/LICENSE-TOKI.md"
cp THIRD_PARTY_LICENSES.md "${WORK_DIR}/THIRD_PARTY_LICENSES.md"

(
  cd "${WORK_DIR}"
  PKGVER="${PKGVER}" makepkg -f --clean
)

mkdir -p target/arch
find "${WORK_DIR}" -maxdepth 1 -type f -name '*.pkg.tar.*' -exec cp {} target/arch/ \;

PACKAGE_SUFFIX="$(sanitize_package_suffix "$(resolve_package_suffix)")"
echo "Applying package suffix: ${PACKAGE_SUFFIX}"
apply_package_suffix "${PACKAGE_SUFFIX}"

echo "Arch package(s):"
find target/arch -maxdepth 1 -type f -name '*.pkg.tar.*' -print | sort
