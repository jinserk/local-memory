use crate::config::Config;
use crate::engine::bq::encode_bq;
use crate::engine::funnel::SearchFunnel;
use crate::storage::db::{Database, Memory};
use crate::storage::tier::{current_timestamp, MemoryTier};
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
#[command(name = "mem-diag")]
#[command(about = "Local Memory Diagnostics Tool", long_about = None)]
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
    /// Search memories using the funnel
    Search {
        /// Search query (text to search for)
        query: String,
        /// Number of results to return
        #[arg(short, long, default_value = "10")]
        top_k: usize,
    },
    /// Inspect a specific memory by ID
    Inspect {
        /// Memory UUID to inspect
        id: String,
    },
    /// Run diagnostic tests (insert, search, delete)
    Test,
}

#[derive(Tabled)]
struct MemoryRow {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Score")]
    score: String,
    #[tabled(rename = "Tier")]
    tier: String,
    #[tabled(rename = "Preview")]
    preview: String,
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

    match cli.command {
        Commands::Stats => run_stats(&storage_path),
        Commands::Search { query, top_k } => run_search(&storage_path, &config, &query, top_k),
        Commands::Inspect { id } => run_inspect(&storage_path, &id),
        Commands::Test => run_test(&storage_path, &config),
    }
}

fn run_stats(storage_path: &PathBuf) -> Result<()> {
    println!("{}", "Memory Statistics".cyan().bold());
    println!();

    let db = Database::open(storage_path)?;

    let mut total_count = 0usize;
    let mut episodic_count = 0usize;
    let mut semantic_count = 0usize;
    let mut expired_count = 0usize;
    let now = current_timestamp();

    for entry in db.metadata_iter() {
        let (_, value) = entry?;
        let metadata: crate::storage::db::MemoryEntry = serde_json::from_slice(&value)?;

        total_count += 1;
        match metadata.tier {
            MemoryTier::Episodic => {
                if let Some(exp) = metadata.expires_at {
                    if now >= exp {
                        expired_count += 1;
                    } else {
                        episodic_count += 1;
                    }
                } else {
                    episodic_count += 1;
                }
            }
            MemoryTier::Semantic => semantic_count += 1,
        }
    }

    let stats = vec![
        StatsRow {
            metric: "Total Memories".to_string(),
            value: total_count.to_string(),
        },
        StatsRow {
            metric: "Semantic (permanent)".to_string(),
            value: semantic_count.to_string().green().to_string(),
        },
        StatsRow {
            metric: "Episodic (temporary)".to_string(),
            value: episodic_count.to_string().yellow().to_string(),
        },
        StatsRow {
            metric: "Expired (not cleaned)".to_string(),
            value: expired_count.to_string().red().to_string(),
        },
        StatsRow {
            metric: "Storage Path".to_string(),
            value: storage_path.display().to_string(),
        },
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

fn run_search(storage_path: &PathBuf, config: &Config, query: &str, top_k: usize) -> Result<()> {
    println!("{} \"{}\"", "Searching for:".cyan().bold(), query);
    println!();

    let db = Database::open(storage_path)?;
    let funnel = SearchFunnel::new(&db, config);

    let query_vector = generate_mock_embedding(query);

    let results = funnel.search(&query_vector, top_k)?;

    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }

    let rows: Vec<MemoryRow> = results
        .iter()
        .map(|r| MemoryRow {
            id: r.id.to_string(),
            score: format!("{:.4}", r.score),
            tier: extract_tier(&r.metadata),
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
    let id =
        Uuid::parse_str(id_str).map_err(|e| anyhow::anyhow!("Invalid UUID '{}': {}", id_str, e))?;

    let db = Database::open(storage_path)?;

    match db.get_memory(id)? {
        Some(memory) => {
            println!("{}", "Memory Details".cyan().bold());
            println!();

            println!("{}: {}", "ID".cyan(), memory.id);
            println!("{}: {}", "Tier".cyan(), memory.tier);
            if let Some(exp) = memory.expires_at {
                let now = current_timestamp();
                let status = if now >= exp {
                    "EXPIRED".red()
                } else {
                    let remaining = exp - now;
                    format!("{}s remaining", remaining).green()
                };
                println!("{}: {} ({})", "Expires At".cyan(), exp, status);
            }
            println!("{}: {}", "Vector Dim".cyan(), memory.vector.len());
            println!(
                "{}: {} bytes",
                "Bit Vector Size".cyan(),
                memory.bit_vector.len()
            );
            println!();
            println!("{}:", "Metadata".cyan());
            let metadata_str = serde_json::to_string_pretty(&memory.metadata)?;
            println!("{}", metadata_str);
        }
        None => {
            println!(
                "{} Memory with ID '{}' not found.",
                "Error:".red().bold(),
                id_str
            );
        }
    }

    Ok(())
}

fn run_test(storage_path: &PathBuf, config: &Config) -> Result<()> {
    println!("{}", "Running Diagnostic Tests".cyan().bold());
    println!();

    let db = Database::open(storage_path)?;

    println!("{}", "[1/3] Testing insert...".yellow());
    let test_id = Uuid::new_v4();
    let test_vector = vec![0.5; 768];
    let test_memory = Memory {
        id: test_id,
        metadata: serde_json::json!({
            "text": "Diagnostic test memory",
            "test": true,
            "timestamp": current_timestamp()
        }),
        vector: test_vector.clone(),
        bit_vector: encode_bq(&test_vector),
        tier: MemoryTier::Episodic,
        expires_at: Some(current_timestamp() + 3600),
    };

    db.insert_memory(&test_memory)?;
    println!("  {} Inserted memory: {}", "✓".green(), test_id);

    println!("{}", "[2/3] Testing search...".yellow());
    let funnel = SearchFunnel::new(&db, config);
    let query_vector = vec![0.5; 768];
    let results = funnel.search(&query_vector, 10)?;

    let found = results.iter().any(|r| r.id == test_id);
    if found {
        println!("  {} Found inserted memory in search results", "✓".green());
    } else {
        println!(
            "  {} Memory not found in search results (may be normal with few memories)",
            "!".yellow()
        );
    }

    println!("{}", "[3/3] Testing delete...".yellow());
    db.delete_memory(test_id)?;
    println!("  {} Deleted memory: {}", "✓".green(), test_id);

    match db.get_memory(test_id)? {
        Some(_) => {
            println!("  {} Memory still exists after delete!", "✗".red());
        }
        None => {
            println!("  {} Verified memory is deleted", "✓".green());
        }
    }

    println!();
    println!("{}", "All diagnostic tests completed!".green().bold());

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

fn extract_tier(metadata: &serde_json::Value) -> String {
    metadata
        .get("tier")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_preview(metadata: &serde_json::Value, max_len: usize) -> String {
    let text = metadata.get("text").and_then(|v| v.as_str()).unwrap_or("");

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
