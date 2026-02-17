use std::path::PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub path: PathBuf,
    pub content: String,
    pub metadata: NoteMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteMetadata {
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

impl Note {
    pub fn new(path: PathBuf, content: String) -> Self {
        Self {
            path,
            content,
            metadata: NoteMetadata {
                title: None,
                tags: Vec::new(),
                created: Utc::now(),
                modified: Utc::now(),
            },
        }
    }

    /// Load a note from a file path, parsing frontmatter if present
    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        
        let (frontmatter, body) = Self::parse_frontmatter(&content);
        let metadata = Self::extract_metadata(frontmatter, &path)?;
        
        Ok(Self {
            path,
            content: body,
            metadata,
        })
    }

    /// Parse YAML frontmatter from markdown content
    /// Returns (frontmatter, body) tuple
    fn parse_frontmatter(content: &str) -> (Option<&str>, String) {
        let trimmed = content.trim_start();
        
        if !trimmed.starts_with("---") {
            return (None, content.to_string());
        }

        // Find the closing ---
        let after_opening = &trimmed[3..];
        if let Some(end_idx) = after_opening.find("\n---") {
            let frontmatter = &after_opening[..end_idx].trim();
            let body = &after_opening[end_idx + 4..].trim_start();
            (Some(frontmatter), body.to_string())
        } else {
            (None, content.to_string())
        }
    }

    /// Extract metadata from frontmatter and file stats
    fn extract_metadata(frontmatter: Option<&str>, path: &PathBuf) -> Result<NoteMetadata> {
        let mut metadata = NoteMetadata {
            title: None,
            tags: Vec::new(),
            created: Utc::now(),
            modified: Utc::now(),
        };

        // Try to get file timestamps
        if let Ok(file_meta) = std::fs::metadata(path) {
            if let Ok(modified) = file_meta.modified() {
                metadata.modified = modified.into();
            }
            if let Ok(created) = file_meta.created() {
                metadata.created = created.into();
            }
        }

        // Parse frontmatter if present
        if let Some(fm) = frontmatter {
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(fm) {
                if let Some(mapping) = yaml.as_mapping() {
                    // Extract title
                    if let Some(title) = mapping.get("title") {
                        metadata.title = title.as_str().map(String::from);
                    }

                    // Extract tags
                    if let Some(tags) = mapping.get("tags") {
                        if let Some(arr) = tags.as_sequence() {
                            metadata.tags = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect();
                        }
                    }
                }
            }
        }

        // If no title in frontmatter, use filename
        if metadata.title.is_none() {
            metadata.title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from);
        }

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
title: Test Note
tags: [rust, coding]
---

# Content here"#;

        let (fm, body) = Note::parse_frontmatter(content);
        assert!(fm.is_some());
        assert!(body.contains("# Content here"));
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "# Just a title\n\nSome content";
        let (fm, body) = Note::parse_frontmatter(content);
        assert!(fm.is_none());
        assert_eq!(body, content);
    }
}
