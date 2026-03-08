#!/usr/bin/env bash
set -euo pipefail

detect_with_getconf() {
  local raw
  raw="$(getconf GNU_LIBC_VERSION 2>/dev/null || true)"
  if [[ -z "${raw}" ]]; then
    return 1
  fi

  local name version
  name="$(awk '{print tolower($1)}' <<<"${raw}")"
  version="$(awk '{print $2}' <<<"${raw}")"

  if [[ -z "${name}" || -z "${version}" ]]; then
    return 1
  fi

  printf '%s%s\n' \
    "$(tr -cd 'a-z0-9' <<<"${name}")" \
    "$(tr -cd '0-9.' <<<"${version}")"
}

detect_with_ldd() {
  local first_line
  first_line="$(ldd --version 2>/dev/null | head -n1 || true)"
  if [[ -z "${first_line}" ]]; then
    return 1
  fi

  local version
  version="$(grep -oE '[0-9]+(\.[0-9]+)+' <<<"${first_line}" | tail -n1 || true)"
  if [[ -z "${version}" ]]; then
    return 1
  fi

  printf 'glibc%s\n' "$(tr -cd '0-9.' <<<"${version}")"
}

if suffix="$(detect_with_getconf)"; then
  echo "${suffix}"
  exit 0
fi

if suffix="$(detect_with_ldd)"; then
  echo "${suffix}"
  exit 0
fi

echo "unable to detect libc version for package suffix" >&2
exit 1
