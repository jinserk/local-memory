use local_memory::storage::SqliteDatabase;
use tempfile::tempdir;

#[test]
fn test_entity_decay() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("decay_entities.db");
    let dim = 768;
    let db = SqliteDatabase::open(&db_path, dim)?;

    // 1. Create an entity
    let id = db.insert_entity_with_namespace("TestEntity", "Person", "Desc", "default")?;
    
    // Check initial decay_factor
    let count = db.count_entities()?;
    assert_eq!(count, 1);

    // 2. Mock time to simulate decay
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    
    // Set last_recalled_at to 90 days ago (half way)
    let secs_90_days = 90 * 24 * 3600;
    let past = now - secs_90_days;
    
    db.set_entity_last_recalled_at_for_testing(id, past)?;

    // Run decay process
    db.process_decay()?;

    // Check decay_factor (should be around 0.5)
    let decay_factor = db.get_entity_decay_factor_for_testing(id)?;
    assert!(decay_factor < 0.51 && decay_factor > 0.49);

    // 3. Recall entity
    db.get_entity_by_name_with_namespace("TestEntity", "default")?;
    
    // Check decay_factor (should be 1.0)
    let decay_factor = db.get_entity_decay_factor_for_testing(id)?;
    assert_eq!(decay_factor, 1.0);

    // 4. Simulate full decay (more than 180 days)
    let secs_200_days = 200 * 24 * 3600;
    let past = now - secs_200_days;
    db.set_entity_last_recalled_at_for_testing(id, past)?;

    db.process_decay()?;

    // Entity should be gone
    let count = db.count_entities()?;
    assert_eq!(count, 0);

    Ok(())
}

#[test]
fn test_forget_entity() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("forget.db");
    let dim = 768;
    let db = SqliteDatabase::open(&db_path, dim)?;

    db.insert_entity_with_namespace("ForgetMe", "Secret", "Classified", "default")?;
    assert_eq!(db.count_entities()?, 1);

    db.forget_entity("ForgetMe", "default")?;
    // After forget, it should still be 1 because process_decay hasn't run yet
    // Actually, count_entities now filters for decay_factor > 0.0, so it should be 0 immediately.
    assert_eq!(db.count_entities()?, 0);

    // Now run process_decay to actually delete from DB
    db.process_decay()?;
    
    // Still 0
    assert_eq!(db.count_entities()?, 0);

    Ok(())
}

#[test]
fn test_relationship_cleanup() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("decay_rel.db");
    let dim = 768;
    let db = SqliteDatabase::open(&db_path, dim)?;

    let id1 = db.insert_entity_with_namespace("A", "P", "D1", "default")?;
    let id2 = db.insert_entity_with_namespace("B", "P", "D2", "default")?;
    
    db.insert_relationship(id1, id2, "knows", "description")?;
    
    let rels = db.list_relationships(10)?;
    assert_eq!(rels.len(), 1);

    // Decay id1 to 0
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let secs_200_days = 200 * 24 * 3600;
    let past = now - secs_200_days;
    db.set_entity_last_recalled_at_for_testing(id1, past)?;

    db.process_decay()?;

    // Relationship should also be gone
    let rels = db.list_relationships(10)?;
    assert_eq!(rels.len(), 0);

    Ok(())
}
