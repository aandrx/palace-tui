use super::{LinkType, Note, NoteMetadata, ParsedLink};
use anyhow::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct NoteLink {
    pub id: i64,
    pub source_note_id: i64,
    pub target_note_id: Option<i64>,
    pub target_path: String,
    pub link_text: Option<String>,
    pub link_type: String,
    pub target_title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NoteLinkStats {
    pub outgoing: usize,
    pub incoming: usize,
}

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

            CREATE TABLE IF NOT EXISTS note_links (
                id INTEGER PRIMARY KEY,
                source_note_id INTEGER NOT NULL,
                target_note_id INTEGER,
                target_path TEXT NOT NULL,
                link_text TEXT,
                link_type TEXT NOT NULL,
                FOREIGN KEY (source_note_id) REFERENCES notes(id) ON DELETE CASCADE,
                UNIQUE(source_note_id, target_path)
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
            CREATE INDEX IF NOT EXISTS idx_note_links_source ON note_links(source_note_id);
            CREATE INDEX IF NOT EXISTS idx_note_links_target ON note_links(target_note_id);
            CREATE INDEX IF NOT EXISTS idx_note_links_path ON note_links(target_path);
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

    /// Get note ID by file path
    pub fn get_note_id_by_path(&self, path: &Path) -> Result<Option<i64>> {
        let result = self.conn.query_row(
            "SELECT id FROM notes WHERE path = ?1",
            [path.to_str().unwrap_or("")],
            |row| row.get(0),
        );

        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get note ID by target path (handles various path formats)
    pub fn get_note_id_by_target_path(&self, target_path: &Path) -> Result<Option<i64>> {
        let target_str = target_path.to_str().unwrap_or("");

        let result = self.conn.query_row(
            "SELECT id FROM notes WHERE path = ?1 OR path = ?2 OR path LIKE ?3",
            [
                target_str,
                target_str,
                &format!(
                    "%/{}/%",
                    target_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ),
            ],
            |row| row.get(0),
        );

        match result {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Store links for a note (replaces existing links)
    pub fn upsert_note_links(&self, source_note_id: i64, links: &[ParsedLink]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM note_links WHERE source_note_id = ?1",
            [source_note_id],
        )?;

        for link in links {
            let target_path = link
                .resolved_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| link.target.clone());

            let link_type_str = match link.link_type {
                LinkType::Wiki => "wiki",
                LinkType::Markdown => "markdown",
            };

            self.conn.execute(
                "INSERT OR IGNORE INTO note_links 
                 (source_note_id, target_note_id, target_path, link_text, link_type)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    source_note_id,
                    link.resolved_path
                        .as_ref()
                        .and_then(|p| self.get_note_id_by_path(p).ok().flatten()),
                    target_path,
                    link.display_text.clone(),
                    link_type_str,
                ),
            )?;
        }

        self.resolve_note_links(source_note_id)?;
        Ok(())
    }

    /// Resolve unresolved links (when target note exists but wasn't found during insert)
    fn resolve_note_links(&self, source_note_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE OR IGNORE note_links 
             SET target_note_id = (
                 SELECT id FROM notes 
                 WHERE notes.path = note_links.target_path 
                    OR notes.title = note_links.target_path
                    OR notes.title || '.md' = note_links.target_path
             )
             WHERE source_note_id = ?1 AND target_note_id IS NULL",
            [source_note_id],
        )?;
        Ok(())
    }

    /// Get all links from a note
    pub fn get_links_from_note(&self, note_id: i64) -> Result<Vec<NoteLink>> {
        let mut stmt = self.conn.prepare(
            "SELECT nl.id, nl.source_note_id, nl.target_note_id, nl.target_path, 
                    nl.link_text, nl.link_type, n.title
             FROM note_links nl
             LEFT JOIN notes n ON n.id = nl.target_note_id
             WHERE nl.source_note_id = ?1",
        )?;

        let links = stmt
            .query_map([note_id], |row| {
                let target_id: Option<i64> = row.get(2)?;
                Ok(NoteLink {
                    id: row.get(0)?,
                    source_note_id: row.get(1)?,
                    target_note_id: target_id,
                    target_path: row.get(3)?,
                    link_text: row.get(4)?,
                    link_type: row.get(5)?,
                    target_title: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(links)
    }

    /// Get all links to a note (backlinks)
    pub fn get_backlinks(&self, note_id: i64) -> Result<Vec<NoteLink>> {
        let mut stmt = self.conn.prepare(
            "SELECT nl.id, nl.source_note_id, nl.target_note_id, nl.target_path,
                    nl.link_text, nl.link_type, n.title
             FROM note_links nl
             LEFT JOIN notes n ON n.id = nl.source_note_id
             WHERE nl.target_note_id = ?1",
        )?;

        let links = stmt
            .query_map([note_id], |row| {
                let source_id: Option<i64> = row.get(1)?;
                Ok(NoteLink {
                    id: row.get(0)?,
                    source_note_id: source_id.unwrap_or(0),
                    target_note_id: row.get(2)?,
                    target_path: row.get(3)?,
                    link_text: row.get(4)?,
                    link_type: row.get(5)?,
                    target_title: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(links)
    }

    /// Get all note links as graph edges
    pub fn get_all_note_links(&self) -> Result<Vec<(i64, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT source_note_id, target_note_id 
             FROM note_links 
             WHERE target_note_id IS NOT NULL",
        )?;

        let links = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(links)
    }

    /// Get link statistics for a note
    pub fn get_note_link_stats(&self, note_id: i64) -> Result<NoteLinkStats> {
        let outgoing: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM note_links 
             WHERE source_note_id = ?1 AND target_note_id IS NOT NULL",
            [note_id],
            |row| row.get(0),
        )?;

        let incoming: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM note_links WHERE target_note_id = ?1",
            [note_id],
            |row| row.get(0),
        )?;

        Ok(NoteLinkStats { outgoing, incoming })
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
        let mut stmt = self
            .conn
            .prepare("SELECT path, title, content, tags, created, modified FROM notes")?;

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
        let count: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Get all note paths with their link counts
    pub fn get_note_paths_with_links(&self) -> Result<Vec<(PathBuf, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.path, COALESCE(COUNT(nl.id), 0) as link_count
             FROM notes n
             LEFT JOIN note_links nl ON n.id = nl.source_note_id
             GROUP BY n.path
             ORDER BY n.path",
        )?;

        let paths = stmt
            .query_map([], |row| {
                let path: String = row.get(0)?;
                let link_count: usize = row.get(1)?;
                Ok((Path::new(&path).to_path_buf(), link_count))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

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
