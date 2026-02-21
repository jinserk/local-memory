use crate::storage::schema::{PARTITION_BIT_INDEX, PARTITION_METADATA, PARTITION_VECTORS};
use anyhow::Result;
use fjall::{Database as FjallDatabase, Keyspace, KeyspaceCreateOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Memory {
    pub id: Uuid,
    pub metadata: serde_json::Value,
    pub vector: Vec<f32>,
    pub bit_vector: Vec<u8>,
}

pub struct Database {
    db: FjallDatabase,
    metadata: Keyspace,
    vectors: Keyspace,
    bit_index: Keyspace,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = FjallDatabase::builder(path).open()?;

        let metadata = db.keyspace(PARTITION_METADATA, KeyspaceCreateOptions::default)?;
        let vectors = db.keyspace(PARTITION_VECTORS, KeyspaceCreateOptions::default)?;
        let bit_index = db.keyspace(PARTITION_BIT_INDEX, KeyspaceCreateOptions::default)?;

        Ok(Self {
            db,
            metadata,
            vectors,
            bit_index,
        })
    }

    pub fn insert_memory(&self, memory: &Memory) -> Result<()> {
        let id_bytes = memory.id.as_bytes();

        let metadata_bytes = serde_json::to_vec(&memory.metadata)?;
        let vector_bytes = bincode::serialize(&memory.vector)?;

        let mut batch = self.db.batch();
        batch.insert(&self.metadata, id_bytes, metadata_bytes);
        batch.insert(&self.vectors, id_bytes, vector_bytes);
        batch.insert(&self.bit_index, id_bytes, &memory.bit_vector);
        batch.commit()?;

        Ok(())
    }

    pub fn get_memory(&self, id: Uuid) -> Result<Option<Memory>> {
        let id_bytes = id.as_bytes();

        let metadata_res = self.metadata.get(id_bytes)?;
        let vector_res = self.vectors.get(id_bytes)?;
        let bit_index_res = self.bit_index.get(id_bytes)?;

        match (metadata_res, vector_res, bit_index_res) {
            (Some(m), Some(v), Some(b)) => {
                let metadata = serde_json::from_slice(&m)?;
                let vector = bincode::deserialize(&v)?;
                let bit_vector = b.to_vec();

                Ok(Some(Memory {
                    id,
                    metadata,
                    vector,
                    bit_vector,
                }))
            }
            _ => Ok(None),
        }
    }

    pub fn delete_memory(&self, id: Uuid) -> Result<()> {
        let id_bytes = id.as_bytes();

        let mut batch = self.db.batch();
        batch.remove(&self.metadata, id_bytes);
        batch.remove(&self.vectors, id_bytes);
        batch.remove(&self.bit_index, id_bytes);
        batch.commit()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_db_crud() -> Result<()> {
        let dir = tempdir()?;
        let db = Database::open(dir.path())?;

        let id = Uuid::new_v4();
        let memory = Memory {
            id,
            metadata: serde_json::json!({"text": "hello world"}),
            vector: vec![1.0, 2.0, 3.0],
            bit_vector: vec![0b10101010],
        };

        db.insert_memory(&memory)?;

        let retrieved = db.get_memory(id)?;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.metadata["text"], "hello world");
        assert_eq!(retrieved.vector, vec![1.0, 2.0, 3.0]);
        assert_eq!(retrieved.bit_vector, vec![0b10101010]);

        db.delete_memory(id)?;
        let retrieved = db.get_memory(id)?;
        assert!(retrieved.is_none());

        Ok(())
    }
}
