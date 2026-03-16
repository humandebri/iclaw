# Site Vision: iclaw Operator Console

## 1. Goal
Internet Computer 上の単一 canister を操作する運用者向け UI を、安心して触れる明るい管理画面として再構成する。対象は既存の React/Vite アプリであり、Stitch では「そのまま本番投入する HTML」ではなく、「ページ再設計の高品質な視覚案」を段階的に生成する。

## 2. Target User
- canister の状態を監視する operator
- runs, schedules, webhooks, memory を横断して保守する開発者
- 日常運用で dark-heavy な画面に疲れている内部利用者

## 3. Integration Notes
- Stitch skill の標準は `site/public/*.html` だが、この repo では `web/public/*.html` を staging 先として使う。
- 生成物はすぐに本番ルーティングへ差し込まず、まず `.stitch/designs/*.html` と `.stitch/designs/*.png` に保持してレビューする。
- 採用する場合のみ、React 側の `web/src/pages/*.tsx` と共通 UI に取り込む。

## 4. Sitemap
- [x] dashboard: `Dashboard.tsx`
- [x] chat: `Chat.tsx`
- [x] runs: `Runs.tsx`
- [x] agents: `Agents.tsx`
- [x] schedules: `Schedules.tsx`
- [x] webhooks: `Webhooks.tsx`
- [x] memory: `Memory.tsx`
- [x] observe: `Observe.tsx`

## 5. Roadmap
- [x] dashboard-refresh: dashboard の再設計案を Stitch で生成する
- [ ] chat-refresh: session selection と conversation area を再配置した chat 案を出す
- [ ] runs-refresh: run list / detail / event timeline の密度を整理した runs 案を出す
- [ ] memory-refresh: memory page を 3 つの明確な操作帯に整理した案を出す
- [ ] webhooks-refresh: secret rotation と rejection review を安全に見せる案を出す

## 6. Creative Freedom
- schedule と webhook を比較できる shared operations hub
- operator onboarding 用の lightweight help page
- blocked run の承認専用サマリー画面
