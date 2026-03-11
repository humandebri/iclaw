//! where: standalone/canister/src/tools/memory_store.rs | what: ICP memory_store tool | why: v1 IC registry must support memory writes without native filesystem tools

use async_trait::async_trait;
use iclaw_standalone_core::memory::{Memory, MemoryCategory};
use iclaw_standalone_core::tools::{Tool, ToolResult};
use std::sync::Arc;

pub struct IcMemoryStoreTool {
    memory: Arc<dyn Memory>,
}

impl IcMemoryStoreTool {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for IcMemoryStoreTool {
    fn name(&self) -> &str {
        "memory_store"
    }

    fn description(&self) -> &str {
        "Store a memory entry in the ICP-safe memory backend."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["key", "content"],
            "properties": {
                "key": { "type": "string" },
                "content": { "type": "string" },
                "category": { "type": "string" },
                "session_id": { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let key = args
            .get("key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let content = args
            .get("content")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let category = parse_category(
            args.get("category")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("conversation"),
        );
        let session_id = args.get("session_id").and_then(serde_json::Value::as_str);
        self.memory
            .store(key, content, category, session_id)
            .await?;
        Ok(ToolResult {
            success: true,
            output: "stored".into(),
            error: None,
        })
    }
}

fn parse_category(raw: &str) -> MemoryCategory {
    match raw.trim() {
        "core" => MemoryCategory::Core,
        "daily" => MemoryCategory::Daily,
        "conversation" | "" => MemoryCategory::Conversation,
        other => MemoryCategory::Custom(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iclaw_standalone_core::memory::MemoryEntry;
    use parking_lot::Mutex;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct StoredCall {
        key: String,
        content: String,
        category: MemoryCategory,
        session_id: Option<String>,
    }

    #[derive(Default)]
    struct TestMemory {
        stored: Mutex<Vec<StoredCall>>,
    }

    #[async_trait]
    impl Memory for TestMemory {
        fn name(&self) -> &str {
            "test-memory"
        }

        async fn store(
            &self,
            key: &str,
            content: &str,
            category: MemoryCategory,
            session_id: Option<&str>,
        ) -> anyhow::Result<()> {
            self.stored.lock().push(StoredCall {
                key: key.to_string(),
                content: content.to_string(),
                category,
                session_id: session_id.map(str::to_string),
            });
            Ok(())
        }

        async fn recall(
            &self,
            _query: &str,
            _limit: usize,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn list(
            &self,
            _category: Option<&MemoryCategory>,
            _session_id: Option<&str>,
        ) -> anyhow::Result<Vec<MemoryEntry>> {
            Ok(Vec::new())
        }

        async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
            Ok(false)
        }

        async fn count(&self) -> anyhow::Result<usize> {
            Ok(0)
        }

        async fn health_check(&self) -> bool {
            true
        }
    }

    #[tokio::test]
    async fn memory_store_tool_defaults_to_conversation_category() {
        let memory = Arc::new(TestMemory::default());
        let tool = IcMemoryStoreTool::new(memory.clone());

        let result = tool
            .execute(serde_json::json!({
                "key": "conversation/session-a/user/1",
                "content": "hello"
            }))
            .await
            .expect("tool execution should succeed");

        assert!(result.success);
        let stored = memory.stored.lock();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].category, MemoryCategory::Conversation);
    }

    #[tokio::test]
    async fn memory_store_tool_accepts_core_category() {
        let memory = Arc::new(TestMemory::default());
        let tool = IcMemoryStoreTool::new(memory.clone());

        let result = tool
            .execute(serde_json::json!({
                "key": "core/user_preferences/style",
                "content": "Prefer concise answers",
                "category": "core"
            }))
            .await
            .expect("tool execution should succeed");

        assert!(result.success);
        let stored = memory.stored.lock();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].category, MemoryCategory::Core);
    }
}
