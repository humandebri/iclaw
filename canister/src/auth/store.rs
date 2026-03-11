// where: iclaw/canister/src/auth/store.rs
// what: Persist operator allowlist state outside the in-memory auth policy
// why: allowlist updates must survive canister upgrades without coupling auth to service startup

use anyhow::Result;
#[cfg(not(test))]
use anyhow::Context;
use candid::Principal;
#[cfg(all(not(test), target_arch = "wasm32"))]
use ic_rusqlite as sqlite_backend;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
use rusqlite as sqlite_backend;
#[cfg(not(test))]
use serde::{Deserialize, Serialize};
#[cfg(not(test))]
use sqlite_backend::params;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
use std::fs;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
use std::path::Path;

#[cfg(not(test))]
const ACCESS_DB_FILE_NAME: &str = "./DB/iclaw_memory.db";
#[cfg(all(not(test), target_arch = "wasm32"))]
const ACCESS_DB_MOUNT_ID: u8 = 120;
#[cfg(not(test))]
const ACCESS_POLICY_STATE_KEY: &str = "operator_allowlist";
#[cfg(not(test))]
const ACCESS_POLICY_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS access_policy_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

#[cfg(not(test))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedAccessPolicy {
    allowed_principals: Vec<String>,
}

#[cfg(test)]
thread_local! {
    static TEST_PERSISTED_POLICY: std::cell::RefCell<Option<Vec<Principal>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub fn clear_test_persisted_allowlist() {
    TEST_PERSISTED_POLICY.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

#[cfg(test)]
pub fn persist_seed_allowlist(_allowed_principals: &[Principal]) -> Result<()> {
    Ok(())
}

#[cfg(not(test))]
pub fn persist_seed_allowlist(allowed_principals: &[Principal]) -> Result<()> {
    save_persisted_allowlist(allowed_principals)
}

#[cfg(test)]
pub fn load_persisted_allowlist() -> Result<Option<Vec<Principal>>> {
    TEST_PERSISTED_POLICY.with(|cell| Ok(cell.borrow().clone()))
}

#[cfg(test)]
pub fn save_persisted_allowlist(allowed_principals: &[Principal]) -> Result<()> {
    TEST_PERSISTED_POLICY.with(|cell| {
        *cell.borrow_mut() = Some(allowed_principals.to_vec());
    });
    Ok(())
}

#[cfg(not(test))]
pub fn load_persisted_allowlist() -> Result<Option<Vec<Principal>>> {
    with_access_db(|conn| {
        let mut statement = conn
            .prepare("SELECT value FROM access_policy_state WHERE key = ?1")
            .context("failed to prepare access policy lookup")?;
        let mut rows = statement
            .query(params![ACCESS_POLICY_STATE_KEY])
            .context("failed to query access policy state")?;
        let Some(row) = rows.next().context("failed to iterate access policy rows")? else {
            return Ok(None);
        };
        let payload: String = row.get(0).context("failed to read access policy value")?;
        let persisted: PersistedAccessPolicy =
            serde_json::from_str(&payload).context("failed to decode access policy payload")?;
        let mut allowed_principals = Vec::new();
        for principal_text in persisted.allowed_principals {
            let principal = Principal::from_text(&principal_text)
                .map_err(|error| anyhow::anyhow!("invalid persisted principal '{principal_text}': {error}"))?;
            allowed_principals.push(principal);
        }
        Ok(Some(allowed_principals))
    })
}

#[cfg(not(test))]
pub fn save_persisted_allowlist(allowed_principals: &[Principal]) -> Result<()> {
    let payload = PersistedAccessPolicy {
        allowed_principals: allowed_principals.iter().map(Principal::to_text).collect(),
    };
    let encoded =
        serde_json::to_string(&payload).context("failed to encode access policy payload")?;
    with_access_db(|conn| {
        conn.execute(
            "INSERT INTO access_policy_state (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![ACCESS_POLICY_STATE_KEY, encoded],
        )
        .context("failed to persist access policy state")?;
        Ok(())
    })
}

#[cfg(target_arch = "wasm32")]
#[cfg(not(test))]
fn with_access_db<T>(f: impl FnOnce(&sqlite_backend::Connection) -> Result<T>) -> Result<T> {
    let mut setup = sqlite_backend::ConnectionConfig::default();
    setup.db_file_name = ACCESS_DB_FILE_NAME.to_string();
    setup.db_file_mount_id = Some(ACCESS_DB_MOUNT_ID);
    sqlite_backend::close_connection();
    sqlite_backend::set_connection_config(setup);
    sqlite_backend::with_connection(|conn| {
        conn.execute_batch(ACCESS_POLICY_SCHEMA_SQL)
            .context("failed to initialize access policy schema")?;
        f(&conn)
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(test))]
fn with_access_db<T>(f: impl FnOnce(&sqlite_backend::Connection) -> Result<T>) -> Result<T> {
    if let Some(parent) = Path::new(ACCESS_DB_FILE_NAME).parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = sqlite_backend::Connection::open(ACCESS_DB_FILE_NAME)
        .with_context(|| format!("failed to open sqlite file {ACCESS_DB_FILE_NAME}"))?;
    conn.execute_batch(ACCESS_POLICY_SCHEMA_SQL)
        .context("failed to initialize access policy schema")?;
    f(&conn)
}
