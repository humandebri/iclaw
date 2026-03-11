//! where: memory/ic_sqlite.rs
//! what: Minimal ICP-oriented SQLite memory backend backed by `ic-rusqlite`
//! why: Keep canister memory support separate from native `SqliteMemory` and its OS-bound features
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
#[cfg(target_arch = "wasm32")]
use ic_rusqlite as sqlite_backend;
use iclaw_standalone_core::memory::{Memory, MemoryCategory, MemoryEntry};
#[cfg(not(target_arch = "wasm32"))]
use rusqlite as sqlite_backend;
use sqlite_backend::{params, Connection};
use std::cmp::Reverse;
use std::collections::BTreeSet;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
#[cfg(target_arch = "wasm32")]
use std::sync::OnceLock;
const DEFAULT_DB_FILE_NAME: &str = "./DB/iclaw_memory.db";
const DEFAULT_DB_MOUNT_ID: u8 = 120;
const INIT_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY,
    key TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    category TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    session_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_ic_memories_category ON memories(category);
CREATE INDEX IF NOT EXISTS idx_ic_memories_session ON memories(session_id);
CREATE INDEX IF NOT EXISTS idx_ic_memories_updated_at ON memories(updated_at);";
static ENTRY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IcSqliteMemoryConfig {
    pub db_file_name: String,
    pub db_file_mount_id: Option<u8>,
}
impl Default for IcSqliteMemoryConfig {
    fn default() -> Self {
        Self {
            db_file_name: DEFAULT_DB_FILE_NAME.to_string(),
            db_file_mount_id: Some(DEFAULT_DB_MOUNT_ID),
        }
    }
}
pub struct IcSqliteMemory {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    config: IcSqliteMemoryConfig,
    #[cfg(not(target_arch = "wasm32"))]
    conn: Mutex<Connection>,
}
impl IcSqliteMemory {
    pub fn new() -> Result<Self> {
        Self::with_config(IcSqliteMemoryConfig::default())
    }

    pub fn with_config(config: IcSqliteMemoryConfig) -> Result<Self> {
        #[cfg(target_arch = "wasm32")]
        let memory = Self { config };

        #[cfg(not(target_arch = "wasm32"))]
        let memory = {
            ensure_parent_dir(&config.db_file_name)?;
            let conn = Connection::open(&config.db_file_name)
                .with_context(|| format!("failed to open sqlite file {}", config.db_file_name))?;
            Self {
                config,
                conn: Mutex::new(conn),
            }
        };

        memory.init_schema()?;
        Ok(memory)
    }

    fn init_schema(&self) -> Result<()> {
        self.with_connection(|conn| {
            conn.execute_batch(INIT_SCHEMA_SQL)?;
            Ok(())
        })
    }

    fn with_connection<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        #[cfg(target_arch = "wasm32")]
        {
            let _guard = connection_lock()
                .lock()
                .map_err(|_| anyhow::anyhow!("ic sqlite memory lock poisoned"))?;
            let mut setup = sqlite_backend::ConnectionConfig::default();
            setup.db_file_name = self.config.db_file_name.clone();
            setup.db_file_mount_id = self.config.db_file_mount_id;
            sqlite_backend::close_connection();
            sqlite_backend::set_connection_config(setup);
            return sqlite_backend::with_connection(|conn| f(&conn));
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self
                .conn
                .lock()
                .map_err(|_| anyhow::anyhow!("ic sqlite memory lock poisoned"))?;
            f(&conn)
        }
    }

    fn row_to_entry(row: &sqlite_backend::Row<'_>) -> sqlite_backend::Result<MemoryEntry> {
        let category: String = row.get(3)?;
        Ok(MemoryEntry {
            id: row.get(0)?,
            key: row.get(1)?,
            content: row.get(2)?,
            category: Self::str_to_category(&category),
            timestamp: row.get(4)?,
            session_id: row.get(5)?,
            score: None,
        })
    }

    fn str_to_category(value: &str) -> MemoryCategory {
        match value {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.to_string()),
        }
    }

    fn score_entry(entry: &MemoryEntry, terms: &[String]) -> Option<f64> {
        let haystack = format!(
            "{}\n{}",
            entry.key.to_ascii_lowercase(),
            entry.content.to_ascii_lowercase()
        );
        let matched_terms = terms
            .iter()
            .filter(|term| haystack.contains(term.as_str()))
            .count();
        if matched_terms == 0 {
            return None;
        }

        let matched = u32::try_from(matched_terms).ok()?;
        let total = u32::try_from(terms.len()).ok()?;
        Some(f64::from(matched) / f64::from(total))
    }

    fn next_entry_id(key: &str, timestamp: &str) -> String {
        let sequence = ENTRY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        format!("{timestamp}:{sequence}:{key}")
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_parent_dir(db_file_name: &str) -> Result<()> {
    if let Some(parent) = Path::new(db_file_name).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn connection_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[async_trait]
impl Memory for IcSqliteMemory {
    fn name(&self) -> &str {
        "ic-sqlite"
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.with_connection(|conn| {
            conn.execute(
                "INSERT INTO memories (id, key, content, category, created_at, updated_at, session_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(key) DO UPDATE SET
                     content = excluded.content,
                     category = excluded.category,
                     updated_at = excluded.updated_at,
                     session_id = excluded.session_id",
                params![
                    Self::next_entry_id(key, &now),
                    key,
                    content,
                    category.to_string(),
                    now,
                    now,
                    session_id
                ],
            )
            .context("failed to store ic sqlite memory")?;
            Ok(())
        })
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|term| term.trim().to_ascii_lowercase())
            .filter(|term| !term.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if terms.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let mut entries = self.list(None, session_id).await?;
        entries.retain_mut(|entry| {
            entry.score = Self::score_entry(entry, &terms);
            entry.score.is_some()
        });
        entries.sort_by_key(|entry| {
            (
                Reverse(entry.score.unwrap_or(0.0).to_bits()),
                Reverse(entry.timestamp.clone()),
            )
        });
        entries.truncate(limit);
        Ok(entries)
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        self.with_connection(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, key, content, category, created_at, session_id
                 FROM memories WHERE key = ?1",
            )?;
            Ok(stmt.query_row(params![key], Self::row_to_entry).ok())
        })
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>> {
        self.with_connection(|conn| {
            let sql = match (category, session_id) {
                (Some(_), Some(_)) => "SELECT id, key, content, category, created_at, session_id FROM memories WHERE category = ?1 AND session_id = ?2 ORDER BY updated_at DESC",
                (Some(_), None) => "SELECT id, key, content, category, created_at, session_id FROM memories WHERE category = ?1 ORDER BY updated_at DESC",
                (None, Some(_)) => "SELECT id, key, content, category, created_at, session_id FROM memories WHERE session_id = ?1 ORDER BY updated_at DESC",
                (None, None) => "SELECT id, key, content, category, created_at, session_id FROM memories ORDER BY updated_at DESC",
            };
            let category_value = category.map(ToString::to_string);
            let rows = match (category_value.as_deref(), session_id) {
                (Some(category_value), Some(session_id)) => {
                    let mut stmt = conn.prepare(sql)?;
                    let rows = stmt
                        .query_map(params![category_value, session_id], Self::row_to_entry)?
                        .collect::<sqlite_backend::Result<Vec<_>>>()?;
                    rows
                }
                (Some(category_value), None) => {
                    let mut stmt = conn.prepare(sql)?;
                    let rows = stmt
                        .query_map(params![category_value], Self::row_to_entry)?
                        .collect::<sqlite_backend::Result<Vec<_>>>()?;
                    rows
                }
                (None, Some(session_id)) => {
                    let mut stmt = conn.prepare(sql)?;
                    let rows = stmt
                        .query_map(params![session_id], Self::row_to_entry)?
                        .collect::<sqlite_backend::Result<Vec<_>>>()?;
                    rows
                }
                (None, None) => {
                    let mut stmt = conn.prepare(sql)?;
                    let rows = stmt
                        .query_map([], Self::row_to_entry)?
                        .collect::<sqlite_backend::Result<Vec<_>>>()?;
                    rows
                }
            };
            Ok(rows)
        })
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        self.with_connection(|conn| {
            Ok(conn.execute("DELETE FROM memories WHERE key = ?1", params![key])? > 0)
        })
    }

    async fn count(&self) -> Result<usize> {
        self.with_connection(|conn| {
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))?;
            usize::try_from(count).context("memory count overflow")
        })
    }

    async fn health_check(&self) -> bool {
        self.with_connection(|conn| {
            let value: i64 = conn.query_row("SELECT 1", [], |row| row.get(0))?;
            Ok(value == 1)
        })
        .unwrap_or(false)
    }
}

#[cfg(test)]
#[path = "ic_sqlite/tests.rs"]
mod tests;
