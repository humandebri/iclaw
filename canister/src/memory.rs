//! where: iclaw/canister/src/memory.rs
//! what: canister memory factory backed by the IcSqliteMemory implementation
//! why: the extracted canister workspace must own its durable memory backend instead of importing root runtime code

use iclaw_core::memory::Memory;
use std::sync::Arc;

mod ic_sqlite;

pub use ic_sqlite::IcSqliteMemory;

pub fn build_memory() -> anyhow::Result<Arc<dyn Memory>> {
    Ok(Arc::new(IcSqliteMemory::new()?))
}
