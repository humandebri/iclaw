// where: iclaw/web/src/lib/chat-state.ts
// what: Pure helpers for chat-session UI state transitions
// why: Session changes must clear browser-only chat bubbles to avoid implying canister-backed history that does not exist

export function sessionChanged(previousSessionId: string, nextSessionId: string): boolean {
  return previousSessionId.trim() !== nextSessionId.trim();
}
