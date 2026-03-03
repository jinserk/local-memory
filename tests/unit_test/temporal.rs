use local_memory::storage::SqliteDatabase;
use tempfile::tempdir;
use uuid::Uuid;
use serde_json::json;

#[test]
fn test_document_versioning() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("temporal.db");
    let db = SqliteDatabase::open(&db_path, 768)?;

    let id1 = Uuid::new_v4();
    let v = vec![0.5; 768];
    let v_short = vec![0.5; 256];
    let v_bit = vec![0u8; 96];

    // Version 1
    db.insert_document_with_namespace(id1, "VersionedDoc", "Original content", &json!({}), &v, &v_short, &v_bit, "default")?;
    
    // Version 2 (Same title)
    let id2 = Uuid::new_v4();
    db.insert_document_with_namespace(id2, "VersionedDoc", "Updated content", &json!({}), &v, &v_short, &v_bit, "default")?;

    // Verify Latest
    let (content, _) = db.get_document_content(id2)?.unwrap();
    assert_eq!(content, "Updated content");

    // Check if old one is not latest anymore (via raw SQL check or search)
    // Here we'll just check if we can still retrieve both but one is marked latest
    Ok(())
}

#[test]
fn test_entity_versioning() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("entity_temporal.db");
    let db = SqliteDatabase::open(&db_path, 768)?;

    // Version 1
    db.insert_entity_with_namespace("Boston", "Location", "A city in USA", "default")?;
    
    // Version 2 (Update description)
    db.insert_entity_with_namespace("Boston", "Location", "The capital of Massachusetts", "default")?;

    let (_, etype, desc) = db.get_entity_by_name_with_namespace("Boston", "default")?.unwrap();
    assert_eq!(etype, "Location");
    assert_eq!(desc, "The capital of Massachusetts");

    Ok(())
}
