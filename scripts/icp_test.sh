#!/usr/bin/env bash
# where: standalone/scripts/icp_test.sh
# what: Run the PocketIC integration suite for the standalone iclaw canister
# why: keep one short command for standalone canister API regression checks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STANDALONE_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${STANDALONE_ROOT}/tests"
npm test
