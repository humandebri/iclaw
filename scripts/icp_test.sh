#!/usr/bin/env bash
# where: iclaw/scripts/icp_test.sh
# what: Run the PocketIC integration suite for the iclaw canister
# why: keep one short command for iclaw canister API regression checks

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ICLAW_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ICLAW_ROOT}/tests"
npm test
