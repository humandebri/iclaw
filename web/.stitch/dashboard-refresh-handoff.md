# dashboard-refresh handoff

## 1. 目的
- このファイルは、Stitch MCP が使える次のセッションで `dashboard-refresh` を生成し、その結果を React 側へ安全に取り込むための最小 handoff です。
- この repo では Stitch 生成物をそのまま本番投入せず、まず `web/.stitch/designs/` に保存してレビューします。

## 2. 次セッションでやること
1. `web/.stitch/next-prompt.md` をそのまま Stitch に渡して `dashboard-refresh` を生成する。
2. 生成結果を以下に保存する。
   - `web/.stitch/designs/dashboard-refresh.html`
   - `web/.stitch/designs/dashboard-refresh.png`
3. HTML と PNG を見て、明るい運用画面として成立しているか確認する。
4. 採用する場合だけ React 側へ取り込む。

## 3. レビュー観点
- dark-heavy な見た目へ戻っていないこと。
- marketing page ではなく、運用 UI の密度と読みやすさを保っていること。
- 画面の主構成が次の 4 ブロックに収まっていること。
  - top summary band
  - metric grid
  - main observation panel
  - right rail
- success / pending / error の色役割が `DESIGN.md` Section 6 と矛盾しないこと。
- sidebar / header / main cards が同じ空気感でそろっていること。
- 右レールに quick actions と allowlist の置き場が確保できること。

## 4. React 取り込み方針
- データ契約は維持し、見た目だけを差し替える。
- 既存のページ境界はなるべく崩さない。
- まずは `Dashboard.tsx` 配下で完結させ、他ページへの波及を避ける。

## 5. 既存コンポーネントとの対応
- top summary band
  - 取り込み先は [DashboardHero.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/pages/dashboard/DashboardHero.tsx)
  - `health`, `currentAgent`, `latestRun`, `latestScheduleFailure`, `latestWebhookFailure`, `blockedRunCount`, `runCount` の責務は維持する
- metric grid
  - 取り込み先は [Dashboard.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/pages/Dashboard.tsx)
  - `StatCard` を拡張するか、新しい dashboard 専用 metric card を追加する
- main observation panel
  - 取り込み先は [DashboardSnapshot.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/pages/dashboard/DashboardSnapshot.tsx)
  - `observe`, `currentAgent`, `latestScheduleRun`, `latestScheduleFailure`, `latestWebhookRun`, `latestWebhookFailure`, `toolPolicies` の入力は固定する
- right rail
  - blocked runs は [BlockedRunsCard.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/pages/dashboard/BlockedRunsCard.tsx)
  - allowlist は [AllowlistPanel.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/components/access/AllowlistPanel.tsx)
  - schedule alerts と quick actions は [Dashboard.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/pages/Dashboard.tsx) の右カラムを調整する

## 6. 実装の優先順位
1. [DashboardHero.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/pages/dashboard/DashboardHero.tsx) を新デザインへ寄せる
2. [DashboardSnapshot.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/pages/dashboard/DashboardSnapshot.tsx) のカード骨格を合わせる
3. [Dashboard.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/pages/Dashboard.tsx) の metric grid と right rail の余白・順序を調整する
4. 必要なら [Card.tsx](/Users/0xhude/Desktop/tes0308/iclaw/web/src/components/ui/Card.tsx) の共通トークンを微調整する

## 7. やらないこと
- canister データの shape を変えない
- dashboard 以外の page を巻き込んで大規模に repaint しない
- Stitch HTML をそのまま埋め込まない
- fallback 用の別 UI を並行維持しない

## 8. 最低限の検証
- `npm test -- Dashboard.test.tsx`
- `npm run build`
- 生成後の見た目確認では、desktop 幅で dashboard の first view と right rail の収まりを見る
- 実ブラウザ確認が必要なら Playwright で dashboard のスクリーンショットを取り、`dashboard-refresh.png` と見比べる
