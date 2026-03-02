use crate::config::Config;
use crate::engine::funnel::SearchFunnel;
use crate::storage::sqlite::SqliteDatabase;
use crate::model::{get_unified_model};
use crate::engine::vectors::{encode_bq, slice_vector};
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
    /// Initialize storage, database, and models (Download/Pull)
    Init,
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

    match cli.command {
        Commands::Init => {
            tokio::runtime::Runtime::new()?.block_on(async {
                run_init(&config).await
            })
        },
        Commands::Stats => {
            tokio::runtime::Runtime::new()?.block_on(async {
                run_stats(&config).await
            })
        },
        Commands::ListEntities { limit } => {
            tokio::runtime::Runtime::new()?.block_on(async {
                run_list_entities(&config, limit).await
            })
        },
        Commands::ListRelations { limit } => {
            tokio::runtime::Runtime::new()?.block_on(async {
                run_list_relations(&config, limit).await
            })
        },
        Commands::Search { query, top_k } => {
            tokio::runtime::Runtime::new()?.block_on(async {
                run_search(&config, &query, top_k).await
            })
        },
        Commands::Inspect { id } => run_inspect(&config.storage_path, &id),
        Commands::Test => {
            tokio::runtime::Runtime::new()?.block_on(async {
                run_test(&config).await
            })
        },
    }
}

async fn run_init(config: &Config) -> Result<()> {
    println!("{}", "Initializing Local Memory...".cyan().bold());
    println!();

    if !config.storage_path.exists() {
        std::fs::create_dir_all(&config.storage_path)?;
        println!("  {} Created storage directory", "✓".green());
    }

    let model = get_unified_model(config).await?;
    model.prepare().await?;
    println!("  {} Model prepared (pulled/downloaded)", "✓".green());

    let db_path = config.storage_path.join("local-memory.db");
    let _db = SqliteDatabase::open(&db_path, model.dimension())?;
    println!("  {} Database initialized", "✓".green());

    println!();
    println!("{}", "Initialization complete!".green().bold());
    Ok(())
}

async fn run_stats(config: &Config) -> Result<()> {
    println!("{}", "Memory Statistics (SQLite)".cyan().bold());
    println!();

    let db_path = config.storage_path.join("local-memory.db");
    if !db_path.exists() {
        println!("{}", "Database file not found. Run 'lmcli init' first.".yellow());
        return Ok(());
    }

    let model = get_unified_model(config).await?;
    let db = SqliteDatabase::open(&db_path, model.dimension())?;

    let stats = vec![
        StatsRow { metric: "Storage Path".to_string(), value: config.storage_path.display().to_string() },
        StatsRow { metric: "Database File".to_string(), value: db_path.display().to_string() },
        StatsRow { metric: "Total Entities".to_string(), value: db.count_entities()?.to_string().green().to_string() }
    ];

    println!("{}", Table::new(stats).with(Modify::new(Rows::new(1..)).with(Alignment::right())).to_string());
    Ok(())
}

async fn run_list_entities(config: &Config, limit: usize) -> Result<()> {
    let db_path = config.storage_path.join("local-memory.db");
    let model = get_unified_model(config).await?;
    let db = SqliteDatabase::open(&db_path, model.dimension())?;
    
    let entities = db.list_entities(limit)?;
    if entities.is_empty() {
        println!("{}", "No entities found.".yellow());
        return Ok(());
    }

    let rows: Vec<EntityRow> = entities.into_iter().map(|(n, t, d)| EntityRow { name: n, entity_type: t, description: d }).collect();
    println!("{}", Table::new(rows).to_string());
    Ok(())
}

async fn run_list_relations(config: &Config, limit: usize) -> Result<()> {
    let db_path = config.storage_path.join("local-memory.db");
    let model = get_unified_model(config).await?;
    let db = SqliteDatabase::open(&db_path, model.dimension())?;
    
    let relations = db.list_relationships(limit)?;
    if relations.is_empty() {
        println!("{}", "No relationships found.".yellow());
        return Ok(());
    }

    let rows: Vec<RelationRow> = relations.into_iter().map(|(s, p, t)| RelationRow { source: s, predicate: p, target: t }).collect();
    println!("{}", Table::new(rows).to_string());
    Ok(())
}

async fn run_search(config: &Config, query: &str, top_k: usize) -> Result<()> {
    let db_path = config.storage_path.join("local-memory.db");
    if !db_path.exists() {
        println!("{}", "Database file not found. Run 'lmcli init' first.".yellow());
        return Ok(());
    }
    
    let model = get_unified_model(config).await?;
    let db = SqliteDatabase::open(&db_path, model.dimension())?;
    let funnel = SearchFunnel::new_sqlite(&db, config);

    println!("{} \"{}\"", "Searching for:".cyan().bold(), query);
    println!();

    let query_vector = model.embed_one(query).await.map_err(|e| anyhow::anyhow!("Embedding failed: {}", e))?;
    let results = funnel.hybrid_search(&query_vector, top_k)?;

    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }

    let rows: Vec<MemoryRow> = results.iter().map(|r| MemoryRow {
        id: r.id.to_string(),
        distance: format!("{:.4}", r.score),
        preview: extract_preview(&r.metadata, 50),
    }).collect();

    println!("{}", Table::new(rows).with(Modify::new(Rows::new(1..)).with(Alignment::left())).to_string());
    Ok(())
}

fn run_inspect(storage_path: &PathBuf, _id_str: &str) -> Result<()> {
    println!("{}", "Inspect is not yet fully implemented for SQLite backend".yellow());
    println!("You can use 'sqlite3' command to inspect the database at:");
    println!("{}", storage_path.join("local-memory.db").display());
    Ok(())
}

async fn run_test(config: &Config) -> Result<()> {
    println!("{}", "Running Diagnostic Tests (SQLite)".cyan().bold());
    println!();

    let db_path = config.storage_path.join("local-memory.db");
    let model = get_unified_model(config).await?;
    let db = SqliteDatabase::open(&db_path, model.dimension())?;

    println!("{}", "[1/2] Testing insert...".yellow());
    let test_id = Uuid::new_v4();
    let dim = model.dimension();
    let v_full = vec![0.5; dim];
    let v_short = slice_vector(&v_full, dim / 3);
    let v_bit = encode_bq(&v_full);
    
    db.insert_document(test_id, "Diag", "Content", &serde_json::json!({"text": "Content", "test": true}), &v_full, &v_short, &v_bit)?;
    println!("  {} Inserted document: {}", "✓".green(), test_id);

    println!("{}", "[2/2] Testing search...".yellow());
    let funnel = SearchFunnel::new_sqlite(&db, config);
    let results = funnel.search(&v_full, 10)?;

    if results.iter().any(|r| r.id == test_id) {
        println!("  {} Found inserted document in search results", "✓".green());
    } else {
        println!("  {} Document not found in search results", "!".yellow());
    }

    Ok(())
}

fn extract_preview(metadata: &serde_json::Value, max_len: usize) -> String {
    let text = metadata.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.len() > max_len { format!("{}...", &text[..max_len]) } else { text.to_string() }
}
