//! where: iclaw/canister/src/tools/memory_recall.rs | what: ICP memory_recall tool | why: v1 IC registry must support memory reads without native search tools

use super::IclawTool;
use async_trait::async_trait;
use iclaw_core::memory::Memory;
use iclaw_core::tools::ToolResult;
use std::sync::Arc;

pub struct IcMemoryRecallTool {
    memory: Arc<dyn Memory>,
}

impl IcMemoryRecallTool {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait(?Send)]
impl IclawTool for IcMemoryRecallTool {
    fn name(&self) -> &str {
        "memory_recall"
    }

    fn description(&self) -> &str {
        "Recall matching memory entries from the ICP-safe memory backend."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer" },
                "session_id": { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let limit = args
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(5) as usize;
        let session_id = args.get("session_id").and_then(serde_json::Value::as_str);
        let results = self.memory.recall(query, limit, session_id).await?;
        Ok(ToolResult {
            success: true,
            output: serde_json::to_string(&results)?,
            error: None,
        })
    }
}
