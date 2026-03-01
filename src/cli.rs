use crate::config::Config;
use crate::engine::funnel::SearchFunnel;
use crate::storage::sqlite::SqliteDatabase;
use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;
use tabled::{
    settings::{object::Rows, Alignment, Modify},
    Table, Tabled,
};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "lmcli")]
#[command(about = "Local Memory Diagnostics Tool (SQLite/EdgeQuake-style)", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Path to storage directory
    #[arg(short, long, global = true)]
    pub storage: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show memory statistics
    Stats,
    /// List extracted entities
    ListEntities {
        /// Max number of entities to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// List extracted relationships
    ListRelations {
        /// Max number of relationships to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Search memories using the hybrid funnel
    Search {
        /// Search query (text to search for)
        query: String,
        /// Number of results to return
        #[arg(short, long, default_value = "10")]
        top_k: usize,
    },
    /// Inspect a specific document by ID
    Inspect {
        /// Document UUID to inspect
        id: String,
    },
    /// Run diagnostic tests (insert, search)
    Test,
}

#[derive(Tabled)]
struct MemoryRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Distance")]
    distance: String,
    #[tabled(rename = "Preview")]
    preview: String,
}

#[derive(Tabled)]
struct EntityRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Type")]
    entity_type: String,
    #[tabled(rename = "Description")]
    description: String,
}

#[derive(Tabled)]
struct RelationRow {
    #[tabled(rename = "Source")]
    source: String,
    #[tabled(rename = "Predicate")]
    predicate: String,
    #[tabled(rename = "Target")]
    target: String,
}

#[derive(Tabled)]
struct StatsRow {
    #[tabled(rename = "Metric")]
    metric: String,
    #[tabled(rename = "Value")]
    value: String,
}

pub fn run(cli: Cli) -> Result<()> {
    let config = Config::load();
    let storage_path = cli.storage.unwrap_or_else(|| config.storage_path.clone());

    // Ensure storage path exists
    if !storage_path.exists() {
        std::fs::create_dir_all(&storage_path)?;
    }

    match cli.command {
        Commands::Stats => run_stats(&storage_path),
        Commands::ListEntities { limit } => run_list_entities(&storage_path, limit),
        Commands::ListRelations { limit } => run_list_relations(&storage_path, limit),
        Commands::Search { query, top_k } => run_search(&storage_path, &config, &query, top_k),
        Commands::Inspect { id } => run_inspect(&storage_path, &id),
        Commands::Test => run_test(&storage_path, &config),
    }
}

fn run_stats(storage_path: &PathBuf) -> Result<()> {
    println!("{}", "Memory Statistics (SQLite)".cyan().bold());
    println!();

    let db_path = storage_path.join("local-memory.db");
    
    // Initialize if missing to avoid "not found" error
    let db = SqliteDatabase::open(&db_path)?;

    let stats = vec![
        StatsRow {
            metric: "Storage Path".to_string(),
            value: storage_path.display().to_string(),
        },
        StatsRow {
            metric: "Database File".to_string(),
            value: db_path.display().to_string(),
        },
        StatsRow {
            metric: "Total Entities".to_string(),
            value: db.count_entities()?.to_string().green().to_string(),
        }
    ];

    let table = Table::new(stats)
        .with(Modify::new(Rows::new(1..)).with(Alignment::right()))
        .to_string();

    println!("{}", table);

    if storage_path.exists() {
        let size = calculate_dir_size(storage_path)?;
        let size_str = format_size(size);
        println!();
        println!("{}: {}", "Storage Size".cyan(), size_str);
    }

    Ok(())
}

fn run_list_entities(storage_path: &PathBuf, limit: usize) -> Result<()> {
    let db_path = storage_path.join("local-memory.db");
    let db = SqliteDatabase::open(&db_path)?;
    
    let entities = db.list_entities(limit)?;
    if entities.is_empty() {
        println!("{}", "No entities found.".yellow());
        return Ok(());
    }

    let rows: Vec<EntityRow> = entities
        .into_iter()
        .map(|(name, t, desc)| EntityRow {
            name,
            entity_type: t,
            description: desc,
        })
        .collect();

    println!("{}", Table::new(rows).to_string());
    Ok(())
}

fn run_list_relations(storage_path: &PathBuf, limit: usize) -> Result<()> {
    let db_path = storage_path.join("local-memory.db");
    let db = SqliteDatabase::open(&db_path)?;
    
    let relations = db.list_relationships(limit)?;
    if relations.is_empty() {
        println!("{}", "No relationships found.".yellow());
        return Ok(());
    }

    let rows: Vec<RelationRow> = relations
        .into_iter()
        .map(|(s, p, t)| RelationRow {
            source: s,
            predicate: p,
            target: t,
        })
        .collect();

    println!("{}", Table::new(rows).to_string());
    Ok(())
}

fn run_search(storage_path: &PathBuf, config: &Config, query: &str, top_k: usize) -> Result<()> {
    println!("{} \"{}\"", "Searching for:".cyan().bold(), query);
    println!();

    let db_path = storage_path.join("local-memory.db");
    let db = SqliteDatabase::open(&db_path)?;
    let funnel = SearchFunnel::new_sqlite(&db, config);

    let query_vector = generate_mock_embedding(query);

    let results = funnel.hybrid_search(&query_vector, top_k)?;

    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }

    let rows: Vec<MemoryRow> = results
        .iter()
        .map(|r| MemoryRow {
            id: r.id.to_string(),
            distance: format!("{:.4}", r.score),
            preview: extract_preview(&r.metadata, 50),
        })
        .collect();

    let table = Table::new(rows)
        .with(Modify::new(Rows::new(1..)).with(Alignment::left()))
        .to_string();

    println!("{}", table);
    println!();
    println!("{} result(s) found.", results.len().to_string().green());

    Ok(())
}

fn run_inspect(storage_path: &PathBuf, id_str: &str) -> Result<()> {
    let _id = Uuid::parse_str(id_str).map_err(|e| anyhow::anyhow!("Invalid UUID '{}': {}", id_str, e))?;

    println!("{}", "Inspect is not yet fully implemented for SQLite backend".yellow());
    println!("You can use 'sqlite3' command to inspect the database at:");
    println!("{}", storage_path.join("local-memory.db").display());

    Ok(())
}

fn run_test(storage_path: &PathBuf, config: &Config) -> Result<()> {
    println!("{}", "Running Diagnostic Tests (SQLite)".cyan().bold());
    println!();

    let db_path = storage_path.join("local-memory.db");
    let db = SqliteDatabase::open(&db_path)?;

    println!("{}", "[1/2] Testing insert...".yellow());
    let test_id = Uuid::new_v4();
    let test_vector = vec![0.5; 768];
    
    db.insert_document(
        test_id, 
        "Diagnostic Test", 
        "This is a diagnostic test content", 
        &serde_json::json!({"text": "This is a diagnostic test content", "test": true}), 
        &test_vector
    )?;
    println!("  {} Inserted document: {}", "✓".green(), test_id);

    println!("{}", "[2/2] Testing search...".yellow());
    let funnel = SearchFunnel::new_sqlite(&db, config);
    let query_vector = vec![0.5; 768];
    let results = funnel.search(&query_vector, 10)?;

    let found = results.iter().any(|r| r.id == test_id);
    if found {
        println!("  {} Found inserted document in search results", "✓".green());
    } else {
        println!(
            "  {} Document not found in search results",
            "!".yellow()
        );
    }

    println!();
    println!("{}", "Diagnostic tests completed!".green().bold());

    Ok(())
}

fn generate_mock_embedding(text: &str) -> Vec<f32> {
    let mut embedding = vec![0.0f32; 768];
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().cycle().take(768).enumerate() {
        embedding[i] = (b as f32 / 255.0) - 0.5;
    }
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for e in &mut embedding {
            *e /= norm;
        }
    }
    embedding
}

fn extract_preview(metadata: &serde_json::Value, max_len: usize) -> String {
    let text = metadata.get("text").and_then(|v| v.as_str()).unwrap_or("");

    if text.is_empty() {
         return "No preview available".to_string();
    }

    if text.len() > max_len {
        format!("{}...", &text[..max_len])
    } else {
        text.to_string()
    }
}

fn calculate_dir_size(path: &PathBuf) -> Result<u64> {
    let mut total_size = 0u64;

    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                total_size += calculate_dir_size(&path)?;
            } else {
                total_size += entry.metadata()?.len();
            }
        }
    }

    Ok(total_size)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
