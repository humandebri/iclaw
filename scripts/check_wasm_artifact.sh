#!/usr/bin/env bash
# where: iclaw/scripts/check_wasm_artifact.sh
# what: Validate the final iclaw Wasm artifacts without rebuilding them
# why: CI needs a single guard that enforces output presence, metadata, and size budgets after build completion

set -euo pipefail

readonly DEFAULT_WASM_PATH="target/ic/iclaw.wasm"
readonly MAX_WASM_BYTES=7000000
readonly MAX_GZIP_BYTES=2500000

WASM_PATH="${1:-${DEFAULT_WASM_PATH}}"
GZIP_PATH="${WASM_PATH}.gz"

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "error: required command '${command_name}' was not found in PATH" >&2
    exit 127
  fi
}

file_size_bytes() {
  local path="$1"
  wc -c <"${path}" | tr -d '[:space:]'
}

require_file() {
  local path="$1"
  if [ ! -f "${path}" ]; then
    echo "error: required artifact '${path}' was not found" >&2
    exit 65
  fi
}

assert_max_size() {
  local path="$1"
  local max_bytes="$2"
  local actual_bytes
  actual_bytes="$(file_size_bytes "${path}")"
  if [ "${actual_bytes}" -gt "${max_bytes}" ]; then
    echo "error: '${path}' is ${actual_bytes} bytes, exceeds limit ${max_bytes}" >&2
    exit 66
  fi
  echo "==> size ok: ${path} (${actual_bytes} bytes)"
}

require_command ic-wasm
require_file "${WASM_PATH}"
require_file "${GZIP_PATH}"

METADATA_OUTPUT="$(mktemp)"
trap 'rm -f "${METADATA_OUTPUT}"' EXIT

ic-wasm "${WASM_PATH}" metadata candid:service >"${METADATA_OUTPUT}"
if [ ! -s "${METADATA_OUTPUT}" ]; then
  echo "error: '${WASM_PATH}' does not contain public candid:service metadata" >&2
  exit 67
fi

echo "==> metadata ok: candid:service"
assert_max_size "${WASM_PATH}" "${MAX_WASM_BYTES}"
assert_max_size "${GZIP_PATH}" "${MAX_GZIP_BYTES}"
