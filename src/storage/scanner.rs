use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Configuration for the note scanner
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub root_path: PathBuf,
    pub respect_gitignore: bool,
    pub max_depth: Option<usize>,
}

impl ScanConfig {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            respect_gitignore: true,
            max_depth: None,
        }
    }
}

/// Scans directories for markdown files
pub struct NoteScanner {
    config: ScanConfig,
}

impl NoteScanner {
    pub fn new(config: ScanConfig) -> Self {
        Self { config }
    }

    /// Scan the configured directory and return all markdown file paths
    pub fn scan(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        let mut builder = WalkBuilder::new(&self.config.root_path);
        builder.git_ignore(self.config.respect_gitignore);

        if let Some(depth) = self.config.max_depth {
            builder.max_depth(Some(depth));
        }

        for result in builder.build() {
            let entry = result.context("Failed to read directory entry")?;
            let path = entry.path();

            // Only include .md and .markdown files
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" || ext == "markdown" {
                        paths.push(path.to_path_buf());
                    }
                }
            }
        }

        Ok(paths)
    }

    /// Check if a file should be indexed based on its extension
    pub fn is_markdown_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            ext == "md" || ext == "markdown"
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_finds_markdown_files() -> Result<()> {
        let temp = TempDir::new()?;
        let root = temp.path();

        // Create test files
        fs::write(root.join("test1.md"), "# Test 1")?;
        fs::write(root.join("test2.markdown"), "# Test 2")?;
        fs::write(root.join("test3.txt"), "Not markdown")?;

        let config = ScanConfig::new(root);
        let scanner = NoteScanner::new(config);
        let paths = scanner.scan()?;

        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.file_name().unwrap() == "test1.md"));
        assert!(paths
            .iter()
            .any(|p| p.file_name().unwrap() == "test2.markdown"));

        Ok(())
    }

    #[test]
    fn test_is_markdown_file() {
        assert!(NoteScanner::is_markdown_file(Path::new("test.md")));
        assert!(NoteScanner::is_markdown_file(Path::new("test.markdown")));
        assert!(!NoteScanner::is_markdown_file(Path::new("test.txt")));
        assert!(!NoteScanner::is_markdown_file(Path::new("test")));
    }
}
