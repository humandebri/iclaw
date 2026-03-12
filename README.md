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
cargo test --manifest-path /Users/0xhude/Desktop/tes0308/iclaw/Cargo.toml
bash /Users/0xhude/Desktop/tes0308/iclaw/scripts/build_ic_canister.sh iclaw-canister target/ic/iclaw.wasm
cd /Users/0xhude/Desktop/tes0308/iclaw/tests && npm test
cd /Users/0xhude/Desktop/tes0308/iclaw/web && npm run build
icp network start -d
icp deploy -e local
icp canister call iclaw health '()' --query -e local
```

## 補足

- canister の外部識別子は `iclaw_ic` を維持しています。
- 生成される Wasm の正規出力先は `target/ic/iclaw.wasm` です。
- UI bindings は `web/src/generated` に生成されます。
- `run_create` は API レベルで成功すると常に `Ok(Run)` を返し、provider 未設定や upstream 失敗は `Run.status = "failed"` と `Run.error` で表現します。
