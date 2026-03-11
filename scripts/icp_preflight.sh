#!/usr/bin/env bash
# where: standalone/scripts/icp_preflight.sh
# what: Validate local prerequisites before standalone canister and PocketIC runs
# why: fail fast on missing toolchain or broken standalone workspace wiring

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STANDALONE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "error: required command '${command_name}' was not found in PATH" >&2
    exit 127
  fi
}

require_command cargo
require_command npm

cd "${STANDALONE_ROOT}"

echo "==> cargo check standalone canister"
cargo check -p iclaw-standalone-canister --target wasm32-wasip1

echo "==> install standalone tests dependencies"
cd "${STANDALONE_ROOT}/tests"
npm install

echo "==> run PocketIC integration tests"
npm test
