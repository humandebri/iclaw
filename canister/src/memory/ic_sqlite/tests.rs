use super::{IcSqliteMemory, IcSqliteMemoryConfig};
use iclaw_standalone_core::memory::{Memory, MemoryCategory};

fn test_memory(name: &str, mount_id: u8) -> IcSqliteMemory {
    IcSqliteMemory::with_config(IcSqliteMemoryConfig {
        db_file_name: format!("./DB/{name}.db"),
        db_file_mount_id: Some(mount_id),
    })
    .unwrap()
}

#[tokio::test]
async fn store_get_and_count_round_trip() {
    let memory = test_memory("store_get_and_count_round_trip", 121);
    memory
        .store("language", "Rust", MemoryCategory::Core, Some("session-a"))
        .await
        .unwrap();
    let entry = memory.get("language").await.unwrap().unwrap();
    assert_eq!(entry.content, "Rust");
    assert_eq!(entry.session_id.as_deref(), Some("session-a"));
    assert_eq!(memory.count().await.unwrap(), 1);
}

#[tokio::test]
async fn store_upserts_without_creating_duplicate_keys() {
    let memory = test_memory("store_upserts_without_creating_duplicate_keys", 122);
    memory
        .store("tone", "concise", MemoryCategory::Core, None)
        .await
        .unwrap();
    let original = memory.get("tone").await.unwrap().unwrap();
    memory
        .store(
            "tone",
            "direct",
            MemoryCategory::Conversation,
            Some("session-b"),
        )
        .await
        .unwrap();
    let updated = memory.get("tone").await.unwrap().unwrap();
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.content, "direct");
    assert_eq!(updated.category, MemoryCategory::Conversation);
    assert_eq!(updated.session_id.as_deref(), Some("session-b"));
    assert_eq!(memory.count().await.unwrap(), 1);
}

#[tokio::test]
async fn list_filters_by_category_and_session() {
    let memory = test_memory("list_filters_by_category_and_session", 123);
    memory
        .store("core", "keep", MemoryCategory::Core, Some("session-x"))
        .await
        .unwrap();
    memory
        .store("daily", "skip", MemoryCategory::Daily, Some("session-y"))
        .await
        .unwrap();
    let entries = memory
        .list(Some(&MemoryCategory::Core), Some("session-x"))
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key, "core");
}

#[tokio::test]
async fn forget_reports_existing_and_missing_keys() {
    let memory = test_memory("forget_reports_existing_and_missing_keys", 124);
    memory
        .store("obsolete", "old", MemoryCategory::Daily, None)
        .await
        .unwrap();
    assert!(memory.forget("obsolete").await.unwrap());
    assert!(!memory.forget("missing").await.unwrap());
}

#[tokio::test]
async fn recall_scores_matches_and_respects_limit() {
    let memory = test_memory("recall_scores_matches_and_respects_limit", 125);
    memory
        .store("rust", "Rust async memory", MemoryCategory::Core, None)
        .await
        .unwrap();
    memory
        .store("sqlite", "SQLite persistence", MemoryCategory::Core, None)
        .await
        .unwrap();
    assert!(memory.recall("", 5, None).await.unwrap().is_empty());
    let entries = memory.recall("rust sqlite", 1, None).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].score.unwrap_or_default() > 0.0);
}

#[tokio::test]
async fn reinitialization_keeps_persisted_rows() {
    let initial = test_memory("reinitialization_keeps_persisted_rows", 126);
    initial
        .store("persisted", "value", MemoryCategory::Core, None)
        .await
        .unwrap();
    let reopened = test_memory("reinitialization_keeps_persisted_rows", 126);
    let entry = reopened.get("persisted").await.unwrap().unwrap();
    assert_eq!(entry.content, "value");
    assert_eq!(reopened.count().await.unwrap(), 1);
    assert!(reopened.health_check().await);
}
