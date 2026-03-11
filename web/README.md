<!-- where: iclaw/web/README.md -->
<!-- what: Local E2E notes for the single-canister caller UI -->
<!-- why: Internet Identity browser tests need a specific local replica workflow that differs from unit tests -->

# iclaw/web E2E

`iclaw/web` には Internet Identity を使うローカル専用の Playwright E2E があります。対象は caller UI の認証・認可導線です。

## 必要ツール

- `icp`
- `node` / `npm`
- Chromium を含む Playwright browser
- ローカル Internet Identity を含む `icp network start -d` が一度は成功している環境

初回だけ browser を入れます。

```bash
cd /path/to/repo/iclaw/web
npx playwright install chromium
```

## 実行

```bash
cd /path/to/repo/iclaw/web
npm run test:e2e
```

headless ではなく確認したいときは次です。

```bash
cd /path/to/repo/iclaw/web
npm run test:e2e:headed
```

## 何を検証するか

- `login -> denied -> principal visible`
- `allowlist update -> reload -> allowed dashboard`
- `logout -> second II account -> denied`
- `allowed state -> chat -> session switch clears browser log`

## allowed / denied の仕組み

`allowed` ケースでは principal を事前に知れないため、最初は placeholder allowlist で canister を deploy します。  
その状態で一度 II ログインすると access denied 画面に principal が出るので、その principal を使って helper が canister allowlist を更新し、同じ principal を許可します。現在の canister には `allowed_principals_get` / `allowed_principals_set` が入っており、運用上の allowlist は upgrade 後も保持されます。

## provider あり / なし

`session switch` テストは `chat()` 成功が必要なので provider 設定が必要です。  
`OPENAI_API_KEY` が未設定でも認証・認可の E2E は動きますが、session switch ケースは skip されます。

必要なら次を設定してください。

```bash
export OPENAI_API_KEY=...
export OPENAI_API_URL=https://api.openai.com/v1
export OPENAI_MODEL=gpt-4o-mini
```

## II fixture の制約

- `@dfinity/internet-identity-playwright` は Playwright セッション内で passkey を再利用します。
- IndexedDB の制約があるため、Playwright の通常の authenticated state 保存には依存していません。
- denied principal の取得は browser storage を読まず、UI 上の `principal:` 表示だけを使っています。
