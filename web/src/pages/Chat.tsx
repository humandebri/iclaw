// where: standalone/web/src/pages/Chat.tsx
// what: Thin chat caller focused on session selection plus compact observation feedback
// why: session_id is the main UX pain point, so the UI keeps it first-class next to prompt input

import { useState } from "react";
import { Send } from "lucide-react";
import { Badge, Card } from "@/components/ui/Card";
import type { ChatMessage, CurrentSession, ObserveViewModel } from "@/types/ui";

export function ChatPage({
  messages,
  pending,
  sessionId,
  sessions,
  setSessionId,
  onSend,
  observe,
}: {
  messages: ChatMessage[];
  pending: boolean;
  sessionId: string;
  sessions: CurrentSession[];
  setSessionId: (value: string) => void;
  onSend: (prompt: string) => Promise<void>;
  observe: ObserveViewModel;
}) {
  const [prompt, setPrompt] = useState("");

  const submit = async () => {
    const trimmed = prompt.trim();
    if (!trimmed) {
      return;
    }
    setPrompt("");
    await onSend(trimmed);
  };

  return (
    <div className="grid gap-6 xl:grid-cols-[1.3fr_0.7fr]">
      <Card title="Chat" subtitle="chat() を session_id とセットで扱う薄い caller">
        <div className="space-y-4">
          <div className="grid gap-3 md:grid-cols-[1fr_auto]">
            <input
              data-tid="chat-session-input"
              value={sessionId}
              onChange={(event) => setSessionId(event.target.value)}
              placeholder="session_id を入力すると継続会話になります"
              className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white outline-none ring-0 transition-colors placeholder:text-slate-500 focus:border-blue-400/40"
            />
            <select
              value=""
              onChange={(event) => {
                if (event.target.value) {
                  setSessionId(event.target.value);
                }
              }}
              className="rounded-xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-slate-200"
            >
              <option value="">recent sessions</option>
              {sessions.map((session) => (
                <option key={session.id} value={session.id}>
                  {session.label}
                </option>
              ))}
            </select>
          </div>

          {!sessionId && (
            <div className="rounded-xl border border-amber-400/20 bg-amber-500/10 px-4 py-3 text-sm text-amber-100">
              session_id 未設定のため、この chat は stateless として扱われます。
            </div>
          )}

          <div className="max-h-[32rem] space-y-3 overflow-y-auto rounded-2xl border border-white/10 bg-slate-950/60 p-4">
            {messages.length === 0 && (
              <p data-tid="chat-empty-state" className="text-sm text-slate-500">最初の prompt を送るとここに会話ログが出ます。</p>
            )}
            {messages.map((message) => (
              <div
                key={message.id}
                data-tid="chat-message"
                data-role={message.role}
                className={`rounded-2xl px-4 py-3 ${
                  message.role === "user" ? "ml-auto max-w-[80%] bg-blue-600 text-white" : "mr-auto max-w-[85%] border border-white/10 bg-slate-900 text-slate-100"
                }`}
              >
                <p className="whitespace-pre-wrap text-sm">{message.content}</p>
                <p className="mt-2 text-xs opacity-70">{new Date(message.timestamp).toLocaleString()}</p>
              </div>
            ))}
          </div>

          <div className="flex gap-3">
            <textarea
              data-tid="chat-prompt-input"
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              placeholder="prompt"
              rows={4}
              className="min-h-28 flex-1 rounded-2xl border border-white/10 bg-slate-950/70 px-4 py-3 text-sm text-white outline-none placeholder:text-slate-500 focus:border-blue-400/40"
            />
            <button
              data-tid="chat-send-button"
              type="button"
              onClick={() => void submit()}
              disabled={pending}
              className="inline-flex w-28 items-center justify-center gap-2 rounded-2xl bg-blue-600 px-4 py-3 text-sm font-medium text-white transition-colors hover:bg-blue-500 disabled:cursor-not-allowed disabled:bg-blue-900/60"
            >
              <Send className="h-4 w-4" />
              {pending ? "Sending" : "Send"}
            </button>
          </div>
        </div>
      </Card>

      <Card title="Compact Observe" subtitle="chat 後の挙動を同じ画面で追う">
        <div className="space-y-4 text-sm text-slate-300">
          <div className="flex flex-wrap gap-2">
            <Badge tone={observe.observation?.conversation_summary_present ? "good" : "warn"}>
              summary {observe.observation?.conversation_summary_present ? "present" : "missing"}
            </Badge>
            <Badge tone="info">turns {observe.observation?.conversation_turn_count ?? 0}</Badge>
          </div>
          <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
            <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Auto promoted</p>
            <div className="mt-3 flex flex-wrap gap-2">
              {(observe.observation?.auto_promoted_keys ?? []).map((key) => (
                <Badge key={key} tone="info">{key}</Badge>
              ))}
              {(observe.observation?.auto_promoted_keys ?? []).length === 0 && (
                <p className="text-sm text-slate-500">no promoted keys</p>
              )}
            </div>
          </div>
          {observe.summary && (
            <div className="rounded-2xl border border-white/10 bg-slate-950/60 p-4">
              <p className="text-xs uppercase tracking-[0.2em] text-slate-500">Conversation summary</p>
              <p className="mt-3 whitespace-pre-wrap text-sm text-slate-200">{observe.summary.content}</p>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
