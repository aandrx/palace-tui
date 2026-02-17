use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use super::{Note, NoteMetadata};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                title TEXT,
                content TEXT NOT NULL,
                tags TEXT,
                created INTEGER NOT NULL,
                modified INTEGER NOT NULL,
                checksum TEXT
            );

            CREATE TABLE IF NOT EXISTS entities (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                embedding BLOB
            );

            CREATE TABLE IF NOT EXISTS relationships (
                source_id INTEGER NOT NULL,
                target_id INTEGER NOT NULL,
                relationship_type TEXT NOT NULL,
                strength REAL NOT NULL,
                FOREIGN KEY (source_id) REFERENCES entities(id),
                FOREIGN KEY (target_id) REFERENCES entities(id),
                PRIMARY KEY (source_id, target_id, relationship_type)
            );

            CREATE TABLE IF NOT EXISTS note_entities (
                note_id INTEGER NOT NULL,
                entity_id INTEGER NOT NULL,
                position INTEGER,
                FOREIGN KEY (note_id) REFERENCES notes(id),
                FOREIGN KEY (entity_id) REFERENCES entities(id),
                PRIMARY KEY (note_id, entity_id)
            );

            CREATE INDEX IF NOT EXISTS idx_notes_path ON notes(path);
            CREATE INDEX IF NOT EXISTS idx_notes_modified ON notes(modified);
            CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name);
            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(type);
            ",
        )?;
        Ok(())
    }

    /// Insert or update a note in the database
    pub fn upsert_note(&self, note: &Note) -> Result<i64> {
        let tags_json = serde_json::to_string(&note.metadata.tags)?;
        
        self.conn.execute(
            "INSERT INTO notes (path, title, content, tags, created, modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(path) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                tags = excluded.tags,
                modified = excluded.modified",
            (
                note.path.to_str().unwrap_or(""),
                &note.metadata.title,
                &note.content,
                tags_json,
                note.metadata.created.timestamp(),
                note.metadata.modified.timestamp(),
            ),
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Delete a note from the database
    pub fn delete_note(&self, path: &Path) -> Result<()> {
        self.conn.execute(
            "DELETE FROM notes WHERE path = ?1",
            [path.to_str().unwrap_or("")],
        )?;
        Ok(())
    }

    /// Get all notes from the database
    pub fn get_all_notes(&self) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT path, title, content, tags, created, modified FROM notes"
        )?;

        let notes = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                let title: Option<String> = row.get(1)?;
                let content: String = row.get(2)?;
                let tags_json: String = row.get(3)?;
                let created_ts: i64 = row.get(4)?;
                let modified_ts: i64 = row.get(5)?;

                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

                Ok(Note {
                    path: Path::new(&path).to_path_buf(),
                    content,
                    metadata: NoteMetadata {
                        title,
                        tags,
                        created: chrono::DateTime::from_timestamp(created_ts, 0)
                            .unwrap_or_default(),
                        modified: chrono::DateTime::from_timestamp(modified_ts, 0)
                            .unwrap_or_default(),
                    },
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(notes)
    }

    /// Count total notes in the database
    pub fn count_notes(&self) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM notes",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::path::PathBuf;

    #[test]
    fn test_database_creation() -> Result<()> {
        let temp = NamedTempFile::new()?;
        let db = Database::new(temp.path())?;
        assert_eq!(db.count_notes()?, 0);
        Ok(())
    }

    #[test]
    fn test_upsert_note() -> Result<()> {
        let temp = NamedTempFile::new()?;
        let db = Database::new(temp.path())?;

        let note = Note::new(PathBuf::from("/test/note.md"), "# Test".to_string());
        db.upsert_note(&note)?;

        assert_eq!(db.count_notes()?, 1);
        Ok(())
    }
}
