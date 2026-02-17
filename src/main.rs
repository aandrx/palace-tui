mod graph;
mod nlp;
mod storage;
mod tui;
mod watcher;

use anyhow::Result;
use std::env;
use std::path::PathBuf;
use storage::Ingester;

fn main() -> Result<()> {
    println!("Palace TUI v0.1.0-alpha");
    println!("Personal Knowledge Graph Builder");
    println!();

    // Get notes directory from args or use current directory
    let notes_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap());

    let db_path = notes_path.join(".palace-tui").join("database.db");
    
    // Create .palace-tui directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!("Notes directory: {}", notes_path.display());
    println!("Database: {}", db_path.display());
    println!();

    // Initialize ingester and index notes
    println!("Scanning for markdown files...");
    let ingester = Ingester::new(notes_path, db_path)?;
    
    let count = ingester.index_all()?;
    println!("Indexed {} notes", count);

    let stats = ingester.stats()?;
    println!();
    println!("Statistics:");
    println!("   Total notes: {}", stats.total_notes);
    println!();
    println!("Phase 1.1 (Note Ingestion) complete!");
    
    Ok(())
}
