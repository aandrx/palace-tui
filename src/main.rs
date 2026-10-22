mod graph;
mod nlp;
mod storage;
mod tree;
mod tui;
mod watcher;

use anyhow::Result;
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    println!("Palace TUI v0.1.0-alpha");
    println!("Personal Knowledge Graph Builder");
    println!();

    let notes_path = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap());

    println!("Notes directory: {}", notes_path.display());
    println!();

    tui::run(notes_path)?;

    Ok(())
}
