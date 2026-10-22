use anyhow::{Context, Result};
use std::path::PathBuf;

use super::{Database, LinkExtractor, Note, NoteScanner, ScanConfig};

/// Coordinates note ingestion: scanning, parsing, and storing notes
pub struct Ingester {
    scanner: NoteScanner,
    database: Database,
    link_extractor: LinkExtractor,
}

impl Ingester {
    pub fn new(notes_path: PathBuf, db_path: PathBuf) -> Result<Self> {
        let config = ScanConfig::new(notes_path);
        let scanner = NoteScanner::new(config);
        let database = Database::new(&db_path).context("Failed to initialize database")?;
        let link_extractor = LinkExtractor::new();

        Ok(Self {
            scanner,
            database,
            link_extractor,
        })
    }

    /// Perform a full scan and index all notes
    pub fn index_all(&self) -> Result<IngestionStats> {
        eprintln!("[DEBUG] Starting scan...");
        let paths = self.scanner.scan().context("Failed to scan for notes")?;
        eprintln!("[DEBUG] Found {} markdown files", paths.len());

        let mut notes_count = 0;
        let mut links_count = 0;
        let total = paths.len();

        for (i, path) in paths.iter().enumerate() {
            if i % 50 == 0 {
                eprintln!("[DEBUG] Processing {}/{} files...", i, total);
            }

            match Note::from_file(path.clone()) {
                Ok(note) => {
                    self.database.upsert_note(&note)?;
                    notes_count += 1;

                    let mut links = self
                        .link_extractor
                        .extract_and_resolve(&note.content, &note.path);
                    for link in &mut links {
                        link.resolve(&note.path);
                    }

                    if let Ok(Some(note_id)) = self.database.get_note_id_by_path(&note.path) {
                        self.database.upsert_note_links(note_id, &links)?;
                        links_count += links.len();
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }

        eprintln!(
            "[DEBUG] Indexing complete: {} notes, {} links",
            notes_count, links_count
        );

        Ok(IngestionStats {
            total_notes: notes_count,
            total_links: links_count,
        })
    }

    /// Index a single note file
    pub fn index_note(&self, path: PathBuf) -> Result<()> {
        let note = Note::from_file(path.clone()).context("Failed to load note")?;
        self.database.upsert_note(&note)?;

        let mut links = self
            .link_extractor
            .extract_and_resolve(&note.content, &note.path);
        for link in &mut links {
            link.resolve(&note.path);
        }

        if let Ok(Some(note_id)) = self.database.get_note_id_by_path(&path) {
            self.database.upsert_note_links(note_id, &links)?;
        }

        Ok(())
    }

    /// Remove a note from the index
    pub fn remove_note(&self, path: PathBuf) -> Result<()> {
        self.database
            .delete_note(&path)
            .context("Failed to delete note from database")?;
        Ok(())
    }

    /// Get statistics about indexed notes
    pub fn stats(&self) -> Result<IngestionStats> {
        let links = self.database.get_all_note_links()?;
        Ok(IngestionStats {
            total_notes: self.database.count_notes()?,
            total_links: links.len(),
        })
    }
}

#[derive(Debug)]
pub struct IngestionStats {
    pub total_notes: usize,
    pub total_links: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_index_all() -> Result<()> {
        let temp_notes = TempDir::new()?;
        let temp_db = TempDir::new()?;

        fs::write(
            temp_notes.path().join("note1.md"),
            "# Note 1\n\nSee [note 2](note2.md)",
        )?;
        fs::write(
            temp_notes.path().join("note2.md"),
            "# Note 2\n\nMore content",
        )?;

        let ingester = Ingester::new(
            temp_notes.path().to_path_buf(),
            temp_db.path().join("test.db"),
        )?;

        let stats = ingester.index_all()?;
        assert_eq!(stats.total_notes, 2);

        let final_stats = ingester.stats()?;
        assert_eq!(final_stats.total_notes, 2);

        Ok(())
    }
}
