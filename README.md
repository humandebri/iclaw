# iclaw

`iclaw` は Internet Computer 上で動く単一 canister 構成のアプリケーションです。Rust workspace の canister 実装、共有 core crate、PocketIC 統合テスト、React ベースの caller UI をこの repo 単体で管理します。

## 構成

- `canister`: ICP canister 本体と build script
- `core`: canister から利用する共有契約・provider/tool 型
- `tests`: PocketIC を使う統合テスト
- `web`: caller UI と E2E / unit test
- `scripts`: canister build と補助スクリプト

## 前提ツール

- Rust toolchain
- `cargo`
- `npm`
- `didc`
- `ic-wasm`
- `wasi2ic`
- 必要に応じて Playwright browser

## よく使うコマンド

```bash
cargo check --manifest-path /Users/0xhude/Desktop/tes0308/iclaw/Cargo.toml
cargo test -p iclaw-core
cargo test -p iclaw-canister --lib
bash /Users/0xhude/Desktop/tes0308/iclaw/scripts/build_ic_canister.sh iclaw-canister target/ic/iclaw.wasm
bash /Users/0xhude/Desktop/tes0308/iclaw/scripts/check_wasm_artifact.sh target/ic/iclaw.wasm
cd /Users/0xhude/Desktop/tes0308/iclaw/tests && npm test
cd /Users/0xhude/Desktop/tes0308/iclaw/web && npm run build
icp network start -d
icp deploy -e local
icp canister call iclaw health '()' --query -e local
```

## CI

- GitHub Actions の PR / push gate は `changes` の判定結果に応じて `rust-fast`、`web-fast`、`tests-fast`、`wasm-guard` を実行します。
- `changes` job が差分を見て、変更に関係ある job だけを実行します。
- `rust-fast` は `cargo fmt --check`、`cargo test`、`cargo check -p iclaw-canister --target wasm32-wasip1` を実行します。
- `web-fast` は `web` 配下で `npm ci`、`npm test`、`npm run build` を実行します。
- `tests-fast` は canister wasm を build したうえで `tests` 配下の PocketIC 統合テストを実行します。
- `wasm-guard` は `target/ic/iclaw.wasm` と `target/ic/iclaw.wasm.gz` を生成し、`candid:service` metadata とサイズ上限を検証します。
- Wasm artifact の upload は失敗時だけに限定しています。

## CI のローカル再現

```bash
cargo fmt --check
cargo test -p iclaw-core
cargo test -p iclaw-canister --lib
cargo check --manifest-path /Users/0xhude/Desktop/tes0308/iclaw/Cargo.toml -p iclaw-canister --target wasm32-wasip1
cd /Users/0xhude/Desktop/tes0308/iclaw/web && npm ci && npm test && npm run build
cd /Users/0xhude/Desktop/tes0308/iclaw/tests && npm ci && npm test
cd /Users/0xhude/Desktop/tes0308/iclaw && bash scripts/build_ic_canister.sh iclaw-canister target/ic/iclaw.wasm
cd /Users/0xhude/Desktop/tes0308/iclaw && bash scripts/check_wasm_artifact.sh target/ic/iclaw.wasm
```

- `tests && npm test` は canister wasm build を含む PocketIC 統合テストの再現手順です。

## 補足

- canister の外部識別子は `iclaw_ic` を維持しています。
- 生成される Wasm の正規出力先は `target/ic/iclaw.wasm` です。
- UI bindings は `web/src/generated` に生成されます。
- `run_create` は API レベルで成功すると常に `Ok(Run)` を返し、provider 未設定や upstream 失敗は `Run.status = "failed"` と `Run.error` で表現します。
- Wasm サイズ上限は `scripts/check_wasm_artifact.sh` の `MAX_WASM_BYTES` と `MAX_GZIP_BYTES` で管理します。
