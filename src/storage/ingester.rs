use anyhow::{Context, Result};
use std::path::PathBuf;

use super::{Database, Note, NoteScanner, ScanConfig};

/// Coordinates note ingestion: scanning, parsing, and storing notes
pub struct Ingester {
    scanner: NoteScanner,
    database: Database,
}

impl Ingester {
    pub fn new(notes_path: PathBuf, db_path: PathBuf) -> Result<Self> {
        let config = ScanConfig::new(notes_path);
        let scanner = NoteScanner::new(config);
        let database = Database::new(&db_path)
            .context("Failed to initialize database")?;

        Ok(Self { scanner, database })
    }

    /// Perform a full scan and index all notes
    pub fn index_all(&self) -> Result<usize> {
        let paths = self.scanner.scan()
            .context("Failed to scan for notes")?;

        let mut count = 0;
        for path in paths {
            match Note::from_file(path.clone()) {
                Ok(note) => {
                    self.database.upsert_note(&note)?;
                    count += 1;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                }
            }
        }

        Ok(count)
    }

    /// Index a single note file
    pub fn index_note(&self, path: PathBuf) -> Result<()> {
        let note = Note::from_file(path)
            .context("Failed to load note")?;
        self.database.upsert_note(&note)?;
        Ok(())
    }

    /// Remove a note from the index
    pub fn remove_note(&self, path: PathBuf) -> Result<()> {
        self.database.delete_note(&path)
            .context("Failed to delete note from database")?;
        Ok(())
    }

    /// Get statistics about indexed notes
    pub fn stats(&self) -> Result<IngestionStats> {
        Ok(IngestionStats {
            total_notes: self.database.count_notes()?,
        })
    }
}

#[derive(Debug)]
pub struct IngestionStats {
    pub total_notes: usize,
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

        // Create test notes
        fs::write(temp_notes.path().join("note1.md"), "# Note 1\n\nContent")?;
        fs::write(temp_notes.path().join("note2.md"), "# Note 2\n\nMore content")?;

        let ingester = Ingester::new(
            temp_notes.path().to_path_buf(),
            temp_db.path().join("test.db"),
        )?;

        let count = ingester.index_all()?;
        assert_eq!(count, 2);

        let stats = ingester.stats()?;
        assert_eq!(stats.total_notes, 2);

        Ok(())
    }
}
