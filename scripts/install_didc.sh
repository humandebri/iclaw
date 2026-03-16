#!/usr/bin/env bash
# where: iclaw/scripts/install_didc.sh
# what: Install the latest didc binary for CI and make it available to current and later steps
# why: canister/build.rs shells into the web build, so didc must resolve reliably from PATH in GitHub Actions

set -euo pipefail

TARGET_DIR="${HOME}/.local/bin"
TARGET_BIN="${TARGET_DIR}/didc"
ORIGINAL_PATH="${PATH}"

mkdir -p "${TARGET_DIR}"
export PATH="${TARGET_DIR}:${PATH}"

if [[ -n "${GITHUB_PATH:-}" ]]; then
  echo "${TARGET_DIR}" >>"${GITHUB_PATH}"
fi

if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "PATH=${TARGET_DIR}:${ORIGINAL_PATH}" >>"${GITHUB_ENV}"
fi

DIDC_URL="$(node <<'NODE'
const https = require("node:https");

const headers = {
  "User-Agent": "iclaw-ci",
  Accept: "application/vnd.github+json",
};

if (process.env.GITHUB_TOKEN) {
  headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
}

https
  .get(
    {
      hostname: "api.github.com",
      path: "/repos/dfinity/candid/releases/latest",
      headers,
    },
    (response) => {
      let body = "";
      response.on("data", (chunk) => {
        body += chunk;
      });
      response.on("end", () => {
        if (response.statusCode !== 200) {
          console.error(`failed to fetch didc release: ${response.statusCode}`);
          process.exit(1);
        }
        const release = JSON.parse(body);
        const asset = release.assets.find((candidate) =>
          /^didc-.*linux/i.test(candidate.name),
        );
        if (!asset) {
          console.error("didc linux asset not found in latest candid release");
          process.exit(1);
        }
        process.stdout.write(asset.browser_download_url);
      });
    },
  )
  .on("error", (error) => {
    console.error(error.message);
    process.exit(1);
  });
NODE
)"

curl -fsSL "${DIDC_URL}" -o "${TARGET_BIN}"
chmod +x "${TARGET_BIN}"
command -v didc >/dev/null 2>&1
didc --version
