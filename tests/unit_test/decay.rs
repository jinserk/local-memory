use local_memory::engine::funnel::SearchFunnel;
use local_memory::config::Config;
use local_memory::storage::SqliteDatabase;
use tempfile::tempdir;
use uuid::Uuid;
use serde_json::json;

#[test]
fn test_time_decay_ranking() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("decay.db");
    let dim = 768;
    let db = SqliteDatabase::open(&db_path, dim)?;
    let config = Config::default();
    let funnel = SearchFunnel::new_sqlite(&db, &config);

    let v = vec![0.5; dim];
    let v_short = vec![0.5; dim/3];
    let v_bit = vec![0u8; dim/8];

    // 1. Old document (10 days ago)
    let id_old = Uuid::new_v4();
    let old_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() - (10 * 24 * 3600);
    
    db.insert_document_with_namespace(id_old, "Old", "Content", &json!({"created_at": old_time}), &v, &v_short, &v_bit, "default")?;

    // 2. New document (Now)
    let id_new = Uuid::new_v4();
    db.insert_document_with_namespace(id_new, "New", "Content", &json!({}), &v, &v_short, &v_bit, "default")?;

    // Search with identical vector
    let results = funnel.search_with_namespace(&v, 10, "default")?;
    
    assert_eq!(results.len(), 2);
    // New one should be first (lower score is better distance, decay increases distance for old ones)
    assert_eq!(results[0].id, id_new);
    assert!(results[1].score > results[0].score);

    Ok(())
}
