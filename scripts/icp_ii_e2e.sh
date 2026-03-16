#!/usr/bin/env bash
# where: iclaw/scripts/icp_ii_e2e.sh
# what: Spin up a temporary icp-cli local network with Internet Identity for browser E2E
# why: Playwright auth tests need a disposable replica plus an API-based allowlist update path

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ICLAW_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEFAULT_II_CANISTER_ID="rdmx6-jaaaa-aaaaa-aaadq-cai"

require_command() {
  local command_name="$1"
  if ! command -v "${command_name}" >/dev/null 2>&1; then
    echo "error: required command '${command_name}' was not found in PATH" >&2
    exit 127
  fi
}

pick_gateway_port() {
  if [[ -n "${ICLAW_E2E_GATEWAY_PORT:-}" ]]; then
    printf '%s\n' "${ICLAW_E2E_GATEWAY_PORT}"
    return
  fi

  local port
  while true; do
    port="$((8100 + RANDOM % 400))"
    if ! lsof -iTCP:"${port}" -sTCP:LISTEN >/dev/null 2>&1; then
      printf '%s\n' "${port}"
      return
    fi
  done
}

provider_init_candid() {
  if [[ -n "${OPENAI_API_KEY:-}" && -n "${OPENAI_API_URL:-}" && -n "${OPENAI_MODEL:-}" ]]; then
    printf 'opt record { api_key = "%s"; api_url = "%s"; default_model = "%s"; timeout_secs = opt 60; }' \
      "${OPENAI_API_KEY}" "${OPENAI_API_URL}" "${OPENAI_MODEL}"
  else
    printf 'null'
  fi
}

write_temp_project() {
  local project_root="$1"
  local gateway_port="$2"
  local provider_candid="$3"

  cat >"${project_root}/icp.yaml" <<EOF
# yaml-language-server: \$schema=https://github.com/dfinity/icp-cli/raw/refs/tags/v0.1.0/docs/schemas/icp-yaml-schema.json

canisters:
  - name: iclaw
    build:
      steps:
        - type: script
          commands:
            - bash ${ICLAW_ROOT}/scripts/build_ic_canister.sh iclaw-canister "\$ICP_WASM_OUTPUT_PATH"
    init_args:
      value: >-
        (opt record {
          provider = ${provider_candid};
          context = null;
          allowed_principals = opt vec {
            principal "2vxsx-fae";
          };
        })
      format: candid

networks:
  - name: local
    mode: managed
    gateway:
      bind: 127.0.0.1
      port: ${gateway_port}
    ii: true
EOF
}

read_state_field() {
  local state_file="$1"
  local field_name="$2"
  node -e 'const fs=require("fs"); const data=JSON.parse(fs.readFileSync(process.argv[1], "utf8")); console.log(data[process.argv[2]] ?? "");' \
    "${state_file}" "${field_name}"
}

setup_environment() {
  local state_file="$1"
  local project_root
  local gateway_port
  local provider_candid
  local provider_configured="false"
  local canister_id
  local base_url
  local gateway_url

  project_root="$(mktemp -d "${TMPDIR:-/tmp}/iclaw-ii-e2e.XXXXXX")"
  gateway_port="$(pick_gateway_port)"
  provider_candid="$(provider_init_candid)"
  if [[ "${provider_candid}" != "null" ]]; then
    provider_configured="true"
  fi

  write_temp_project "${project_root}" "${gateway_port}" "${provider_candid}"
  icp network start -d --project-root-override "${project_root}" >/dev/null
  icp deploy -e local -y --project-root-override "${project_root}" >/dev/null

  canister_id="$(node -e 'const fs=require("fs"); const ids=JSON.parse(fs.readFileSync(process.argv[1], "utf8")); console.log(ids.iclaw);' "${project_root}/.icp/cache/mappings/local.ids.json")"
  gateway_url="http://127.0.0.1:${gateway_port}"
  base_url="http://${canister_id}.localhost:${gateway_port}/?canisterId=${canister_id}"

  cat >"${state_file}" <<EOF
{
  "baseUrl": "${base_url}",
  "canisterId": "${canister_id}",
  "gatewayPort": "${gateway_port}",
  "gatewayUrl": "${gateway_url}",
  "iiCanisterId": "${DEFAULT_II_CANISTER_ID}",
  "projectRoot": "${project_root}",
  "providerConfigured": ${provider_configured}
}
EOF

  cat "${state_file}"
}

allow_principal() {
  local state_file="$1"
  local principal_text="$2"
  local project_root

  project_root="$(read_state_field "${state_file}" "projectRoot")"
  icp canister call iclaw allowed_principals_set \
    "(record { allowed_principals = vec { principal \"2vxsx-fae\"; principal \"${principal_text}\" } })" \
    -e local \
    --identity anonymous \
    --project-root-override "${project_root}" >/dev/null
}

teardown_environment() {
  local state_file="$1"
  local project_root

  if [[ ! -f "${state_file}" ]]; then
    exit 0
  fi

  project_root="$(read_state_field "${state_file}" "projectRoot")"
  if [[ -n "${project_root}" && -d "${project_root}" ]]; then
    icp network stop --project-root-override "${project_root}" >/dev/null 2>&1 || true
    rm -rf "${project_root}"
  fi
  rm -f "${state_file}"
}

main() {
  require_command icp
  require_command node
  require_command lsof

  local command="${1:-}"
  case "${command}" in
    setup)
      [[ $# -eq 2 ]] || { echo "usage: $0 setup <state-file>" >&2; exit 64; }
      setup_environment "$2"
      ;;
    allow-principal)
      [[ $# -eq 3 ]] || { echo "usage: $0 allow-principal <state-file> <principal>" >&2; exit 64; }
      allow_principal "$2" "$3"
      ;;
    teardown)
      [[ $# -eq 2 ]] || { echo "usage: $0 teardown <state-file>" >&2; exit 64; }
      teardown_environment "$2"
      ;;
    *)
      echo "usage: $0 <setup|allow-principal|teardown> ..." >&2
      exit 64
      ;;
  esac
}

main "$@"
