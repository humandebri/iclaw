//! where: standalone/canister/src/tools/mod.rs | what: ICP-only tool registry | why: Track E must prove the IC crate only exposes the v1-safe tool surface

mod http_request;
mod memory_forget;
mod memory_recall;
mod memory_store;

use crate::types::ProviderConfig;
use iclaw_standalone_core::memory::Memory;
use iclaw_standalone_core::tools::Tool;
use std::sync::Arc;

pub use http_request::IcHttpRequestTool;
pub use memory_forget::IcMemoryForgetTool;
pub use memory_recall::IcMemoryRecallTool;
pub use memory_store::IcMemoryStoreTool;

pub fn ic_tools(
    memory: Option<Arc<dyn Memory>>,
    provider_config: Option<&ProviderConfig>,
) -> Vec<Box<dyn Tool>> {
    let mut tools: Vec<Box<dyn Tool>> = Vec::new();
    if let Some(memory) = memory {
        tools.push(Box::new(IcMemoryStoreTool::new(memory.clone())));
        tools.push(Box::new(IcMemoryRecallTool::new(memory.clone())));
        tools.push(Box::new(IcMemoryForgetTool::new(memory)));
    }
    tools.push(Box::new(IcHttpRequestTool::new(provider_config)));
    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use iclaw_standalone_core::memory::{Memory, MemoryCategory, MemoryEntry};

    #[derive(Default)]
    struct RegistryTestMemory;

    #[async_trait]
    impl Memory for RegistryTestMemory {
        fn name(&self) -> &str {
            "registry-test-memory"
        }

        async fn store(
            &self,
            _key: &str,
            _content: &str,
            _category: MemoryCategory,
            _session_id: Option<&str>,
        ) -> anyhow::Result<()> {
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

    #[test]
    fn registry_contains_only_v1_safe_tools() {
        let names: Vec<String> = ic_tools(Some(Arc::new(RegistryTestMemory)), None)
            .into_iter()
            .map(|tool| tool.name().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                "memory_store",
                "memory_recall",
                "memory_forget",
                "http_request"
            ]
        );
    }

    #[test]
    fn registry_without_memory_keeps_http_request_tool() {
        let names: Vec<String> = ic_tools(None, None)
            .into_iter()
            .map(|tool| tool.name().to_string())
            .collect();

        assert_eq!(names, vec!["http_request"]);
    }
}
