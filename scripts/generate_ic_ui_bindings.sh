#!/usr/bin/env bash
# where: standalone/scripts/generate_ic_ui_bindings.sh
# what: Generate browser-facing iclaw bindings from the standalone canister DID
# why: the extracted web UI must stay in sync with the standalone canister contract

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STANDALONE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUTPUT_DIR="${STANDALONE_ROOT}/web/src/generated"
DID_PATH="${STANDALONE_ROOT}/canister/iclaw_ic.did"

if ! command -v didc >/dev/null 2>&1; then
  echo "error: didc is required to generate UI bindings" >&2
  exit 127
fi

mkdir -p "${OUTPUT_DIR}"

didc bind "${DID_PATH}" -t js >"${OUTPUT_DIR}/iclaw.did.js"
didc bind "${DID_PATH}" -t ts >"${OUTPUT_DIR}/iclaw.did.d.ts"
