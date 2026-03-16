---
page: chat-refresh
---
Redesign the iclaw chat page as a calm, bright desktop operator workspace. This page is for live session selection, message review, and tool-aware response handling inside a single Internet Computer canister console. It should feel operational and trustworthy, not like a consumer messenger.

**DESIGN SYSTEM (REQUIRED):**
- 明るい管理画面として構成する。背景は `#f4f4f5` を基準にし、`#fef3c7` と `#e2e8f0` の大きなぼかしをうっすら重ねる。
- メインコンテナとカードは半透明の白ガラス面にする。`#ffffff` ベース、薄い境界線は `#e4e4e7`、影は柔らかく短く、重いドロップシャドウは避ける。
- 主要テキストは `#18181b`、補助テキストは `#52525b` を使う。黒ベタや高コントラストの暗色背景は使わない。
- ボタンは pill 形状。主操作は引き締まった濃色単色、補助操作は白面と薄い境界線。破壊的操作だけ `#f43f5e` 系で分離する。
- status pill は success=`#10b981`、pending=`#f59e0b`、error=`#f43f5e` の3系統に固定し、いずれも淡い背景色と組み合わせる。
- サイドバーは独立した白ガラスカードとして扱い、選択中ナビは塗りつぶしではなく、濃い文字と軽い境界の強調で示す。
- 入力欄と textarea は白ベース、薄いグレー境界線、やさしいフォーカスリング。強い発光やネオン表現は使わない。
- 画面全体の印象は「監視室」ではなく「安全で見通しのよい作業卓」。落ち着き、安心感、清潔感を優先する。

**Page Structure:**
1. A left column for session selection with clear active state, unread changes, and status pills for agent/tool state.
2. A main conversation area with message history, compact tool activity markers, and a soft but readable operator composer.
3. A top context band showing selected session, active agent, current tools, and latest observation summary.
4. A right rail for session facts, recent snippets, approval-required actions, and memory/context highlights.
5. Consistent sidebar and top navigation styling that matches the bright design system.

**Implementation Notes:**
- This is a DESKTOP page.
- Design for a React/Vite control panel, not a static landing page.
- The page should support dense operator work: switching sessions quickly, reading assistant output, and noticing tool/approval state without visual noise.
- Prefer operational readability over decorative complexity.
