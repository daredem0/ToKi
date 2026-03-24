#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_ROOT}"

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

if [[ "${TOKI_USE_PREBUILT:-0}" == "1" ]]; then
  echo "Using prebuilt release binaries from target/release."
  ensure_prebuilt_inputs
else
  cargo build --locked --release -p toki-runtime -p toki-editor
fi

APPIMAGE_TOOL="${REPO_ROOT}/target/tools/appimagetool-x86_64.AppImage"
mkdir -p target/tools target/appimage
if [[ ! -f "${APPIMAGE_TOOL}" ]]; then
  curl -L \
    -o "${APPIMAGE_TOOL}" \
    https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "${APPIMAGE_TOOL}"
fi

PKGVER="$(workspace_version)"
if [[ -z "${PKGVER}" ]]; then
  echo "Unable to determine workspace version from Cargo.toml" >&2
  exit 1
fi

APPDIR="${REPO_ROOT}/target/appimage/ToKi-Editor.AppDir"
rm -rf "${APPDIR}"
mkdir -p \
  "${APPDIR}/usr/bin" \
  "${APPDIR}/usr/share/applications" \
  "${APPDIR}/usr/share/licenses"

install -m755 target/release/toki-editor "${APPDIR}/usr/bin/toki-editor"
install -m755 target/release/toki-runtime "${APPDIR}/usr/bin/toki-runtime"
install -m644 packaging/toki-editor.desktop "${APPDIR}/toki-editor.desktop"
install -m644 packaging/toki-editor.desktop "${APPDIR}/usr/share/applications/toki-editor.desktop"
install -m644 packaging/toki-editor.png "${APPDIR}/toki-editor.png"
install -m644 LICENSE-TOKI.md "${APPDIR}/usr/share/licenses/LICENSE-TOKI.md"
install -m644 THIRD_PARTY_LICENSES.md "${APPDIR}/usr/share/licenses/THIRD_PARTY_LICENSES.md"

cat > "${APPDIR}/AppRun" <<'EOF'
#!/usr/bin/env bash
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "${HERE}/usr/bin/toki-editor" "$@"
EOF
chmod +x "${APPDIR}/AppRun"

APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "${APPIMAGE_TOOL}" "${APPDIR}" \
  "target/appimage/ToKi-Editor-${PKGVER}-x86_64.AppImage"

echo "AppImage package(s):"
find target/appimage -maxdepth 1 -type f -name '*.AppImage' -print | sort
