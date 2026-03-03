use local_memory::storage::registry::Registry;
use tempfile::tempdir;

#[test]
fn test_registry_registration() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let db_path = dir.path().join("registry.db");
    
    let registry = Registry::open(&db_path)?;
    
    registry.register_project("/path/to/project", "/path/to/db")?;
    
    let projects = registry.list_projects()?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].0, "/path/to/project");
    assert_eq!(projects[0].1, "/path/to/db");
    
    // Update existing
    registry.register_project("/path/to/project", "/new/path/to/db")?;
    let projects = registry.list_projects()?;
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].1, "/new/path/to/db");

    Ok(())
}
