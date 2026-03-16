//! where: iclaw/canister/src/tools/memory_forget.rs | what: ICP memory_forget tool | why: v1 IC registry must support memory deletion without native storage hooks

use super::IclawTool;
use async_trait::async_trait;
use iclaw_core::memory::Memory;
use iclaw_core::tools::ToolResult;
use std::sync::Arc;

pub struct IcMemoryForgetTool {
    memory: Arc<dyn Memory>,
}

impl IcMemoryForgetTool {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait(?Send)]
impl IclawTool for IcMemoryForgetTool {
    fn name(&self) -> &str {
        "memory_forget"
    }

    fn description(&self) -> &str {
        "Delete a memory entry from the ICP-safe memory backend."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["key"],
            "properties": {
                "key": { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let key = args
            .get("key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let deleted = self.memory.forget(key).await?;
        Ok(ToolResult {
            success: deleted,
            output: if deleted { "deleted" } else { "not_found" }.into(),
            error: None,
        })
    }
}
