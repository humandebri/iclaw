// where: iclaw/web/src/pages/Agents.tsx
// what: Minimal agent control-plane page for selecting and inspecting durable agent records
// why: operators need to see enabled tools and approval posture before launching runs

import { Badge, Card } from "@/components/ui/Card";
import type { AgentsViewModel } from "@/types/ui";

export function AgentsPage({
  agents,
  currentAgentId,
  onSelectAgent,
}: {
  agents: AgentsViewModel;
  currentAgentId: string;
  onSelectAgent: (agentId: string) => void;
}) {
  return (
    <div className="grid gap-6 xl:grid-cols-[0.9fr_1.1fr]">
      <Card title="Agents" subtitle="run 実行前に選べる agent 一覧">
        <div className="space-y-3">
          {agents.items.map((agent) => (
            <button
              key={agent.id}
              type="button"
              onClick={() => onSelectAgent(agent.id)}
              className={`block w-full rounded-2xl border px-4 py-3 text-left transition-colors ${
                currentAgentId === agent.id
                  ? "border-sky-300 bg-sky-50"
                  : "border-zinc-200 bg-zinc-50/70 hover:bg-white"
              }`}
            >
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-sm font-medium text-zinc-900">{agent.name}</p>
                  <p className="mt-2 text-sm text-zinc-600">{agent.description}</p>
                </div>
                <Badge tone={agent.status === "active" ? "good" : "warn"}>{agent.status}</Badge>
              </div>
            </button>
          ))}
        </div>
      </Card>

      <Card title="Agent Detail" subtitle="enabled tools と approval 状態">
        {!agents.selectedAgent && <p className="text-sm text-zinc-500">agent を選ぶと詳細が見えます。</p>}
        {agents.selectedAgent && (
          <div className="space-y-4 text-sm text-zinc-600">
            <div className="flex flex-wrap gap-2">
              <Badge tone={agents.selectedAgent.requires_tool_approval ? "warn" : "good"}>
                approval {agents.selectedAgent.requires_tool_approval ? "required" : "not required"}
              </Badge>
              <Badge tone="info">{agents.selectedAgent.enabled_tool_names.length} enabled tools</Badge>
            </div>
            <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
              <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">System Prompt Override</p>
              <p className="mt-3 whitespace-pre-wrap text-sm text-zinc-700">
                {agents.selectedAgent.system_prompt_override[0] ?? "none"}
              </p>
            </div>
            <div className="rounded-2xl border border-zinc-200 bg-zinc-50/70 p-4">
              <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">Enabled Tools</p>
              <div className="mt-3 flex flex-wrap gap-2">
                {agents.selectedAgent.enabled_tool_names.map((toolName) => {
                  const policy = agents.toolPolicies.find((entry) => entry.tool_name === toolName);
                  return (
                    <Badge key={toolName} tone={policy && (!policy.enabled || policy.requires_approval) ? "warn" : "info"}>
                      {toolName}
                    </Badge>
                  );
                })}
              </div>
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
