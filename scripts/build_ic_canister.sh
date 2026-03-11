#!/usr/bin/env bash
# where: iclaw/scripts/build_ic_canister.sh
# what: Build the iclaw canister and canonicalize the Wasm outputs for local tests
# why: the extracted workspace needs a deterministic build path without depending on the root repo layout

set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 <cargo-package> <output-wasm>" >&2
  exit 64
fi

PACKAGE_NAME="$1"
OUTPUT_WASM="$2"
TARGET_TRIPLE="wasm32-wasip1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ICLAW_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
if [[ "${OUTPUT_WASM}" = /* ]]; then
  OUTPUT_PATH="${OUTPUT_WASM}"
else
  OUTPUT_PATH="${ICLAW_ROOT}/${OUTPUT_WASM}"
fi
OUTPUT_DIR="$(dirname "${OUTPUT_PATH}")"
PACKAGE_WASM_NAME="${PACKAGE_NAME//-/_}.wasm"
SOURCE_WASM="${ICLAW_ROOT}/target/${TARGET_TRIPLE}/release/${PACKAGE_WASM_NAME}"
TMP_WASM="${OUTPUT_PATH%.wasm}.tmp.wasm"
GZIP_OUTPUT_PATH="${OUTPUT_PATH}.gz"
CANONICAL_OUTPUT_PATH="${ICLAW_ROOT}/target/ic/iclaw.wasm"
CANONICAL_GZIP_OUTPUT_PATH="${CANONICAL_OUTPUT_PATH}.gz"
DID_PATH="${ICLAW_ROOT}/canister/iclaw_ic.did"

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "error: required command '${command_name}' was not found in PATH" >&2
    exit 127
  fi
}

require_command cargo
require_command ic-wasm
require_command wasi2ic
require_command gzip

mkdir -p "${OUTPUT_DIR}"

  echo "==> building ${PACKAGE_NAME} for ${TARGET_TRIPLE}"
cargo build \
  --manifest-path "${ICLAW_ROOT}/Cargo.toml" \
  --locked \
  -p "${PACKAGE_NAME}" \
  --target "${TARGET_TRIPLE}" \
  --release

if [ ! -f "${SOURCE_WASM}" ]; then
  echo "error: expected build artifact '${SOURCE_WASM}' was not produced" >&2
  exit 65
fi

cp "${SOURCE_WASM}" "${TMP_WASM}"

echo "==> converting WASI imports to IC system API with wasi2ic"
wasi2ic "${TMP_WASM}" "${TMP_WASM}"

echo "==> embedding candid metadata from ${DID_PATH}"
ic-wasm \
  "${TMP_WASM}" \
  -o "${TMP_WASM}" \
  metadata candid:service \
  -f "${DID_PATH}" \
  -v public

echo "==> shrinking converted wasm with ic-wasm"
ic-wasm "${TMP_WASM}" -o "${OUTPUT_PATH}" shrink

rm -f "${TMP_WASM}"
gzip -9 -c "${OUTPUT_PATH}" >"${GZIP_OUTPUT_PATH}"

mkdir -p "$(dirname "${CANONICAL_OUTPUT_PATH}")"
if [[ "${OUTPUT_PATH}" != "${CANONICAL_OUTPUT_PATH}" ]]; then
  cp "${OUTPUT_PATH}" "${CANONICAL_OUTPUT_PATH}"
  cp "${GZIP_OUTPUT_PATH}" "${CANONICAL_GZIP_OUTPUT_PATH}"
else
  echo "==> canonical output already matches ${OUTPUT_PATH}"
fi

echo "==> wrote ${OUTPUT_PATH}"
echo "==> wrote ${GZIP_OUTPUT_PATH}"
if [[ "${OUTPUT_PATH}" != "${CANONICAL_OUTPUT_PATH}" ]]; then
  echo "==> copied ${OUTPUT_PATH} to ${CANONICAL_OUTPUT_PATH}"
  echo "==> copied ${GZIP_OUTPUT_PATH} to ${CANONICAL_GZIP_OUTPUT_PATH}"
fi
