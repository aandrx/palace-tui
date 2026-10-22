use regex::Regex;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedLink {
    pub target: String,
    pub display_text: Option<String>,
    pub link_type: LinkType,
    pub resolved_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    Wiki,
    Markdown,
}

impl ParsedLink {
    pub fn wiki(target: String) -> Self {
        Self {
            target,
            display_text: None,
            link_type: LinkType::Wiki,
            resolved_path: None,
        }
    }

    pub fn markdown(target: String, display: Option<String>) -> Self {
        Self {
            target,
            display_text: display,
            link_type: LinkType::Markdown,
            resolved_path: None,
        }
    }

    pub fn resolve(&mut self, source_path: &Path) {
        if self.resolved_path.is_some() {
            return;
        }

        let target = self.target.trim();
        if target.is_empty() {
            return;
        }

        let resolved = if target.starts_with('/') {
            PathBuf::from(target)
        } else if target.starts_with("./") || target.starts_with("../") {
            source_path
                .parent()
                .map(|p| p.join(target))
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| PathBuf::from(target))
        } else {
            source_path
                .parent()
                .map(|p| p.join(target))
                .unwrap_or_else(|| PathBuf::from(target))
        };

        let normalized = normalize_path(&resolved);
        self.resolved_path = Some(normalized);
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            _ => components.push(component),
        }
    }
    components.iter().collect()
}

pub struct LinkExtractor {
    wiki_link_regex: Regex,
    markdown_link_regex: Regex,
}

impl LinkExtractor {
    pub fn new() -> Self {
        Self {
            wiki_link_regex: Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").unwrap(),
            markdown_link_regex: Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap(),
        }
    }

    pub fn extract_links(&self, content: &str) -> Vec<ParsedLink> {
        let mut links = Vec::new();

        for cap in self.wiki_link_regex.captures_iter(content) {
            let target = cap.get(1).map(|m| m.as_str().to_string()).unwrap();
            let display = cap.get(0).map(|m| m.as_str().to_string());

            let mut link = ParsedLink::wiki(target);
            link.display_text = display;
            links.push(link);
        }

        for cap in self.markdown_link_regex.captures_iter(content) {
            let display = cap.get(1).map(|m| m.as_str().to_string());
            let target = cap
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            if is_markdown_url(&target) {
                continue;
            }

            let mut link = ParsedLink::markdown(target, display);
            links.push(link);
        }

        links
    }

    pub fn extract_and_resolve(&self, content: &str, source_path: &Path) -> Vec<ParsedLink> {
        let mut links = self.extract_links(content);
        for link in &mut links {
            link.resolve(source_path);
        }
        links
    }
}

impl Default for LinkExtractor {
    fn default() -> Self {
        Self::new()
    }
}

fn is_markdown_url(target: &str) -> bool {
    target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("ftp://")
        || target.starts_with("file://")
        || target.starts_with('#')
        || target.starts_with("javascript:")
}

pub fn extract_title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace('-', " ").replace('_', " "))
        .unwrap_or_else(|| "Untitled".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_wiki_links() {
        let extractor = LinkExtractor::new();

        let content = "Check [[My Note]] and [[Another Note|Display Text]]";
        let links = extractor.extract_links(content);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "My Note");
        assert!(matches!(links[0].link_type, LinkType::Wiki));
        assert_eq!(links[1].target, "Another Note");
        assert!(matches!(links[1].link_type, LinkType::Wiki));
    }

    #[test]
    fn test_extract_markdown_links() {
        let extractor = LinkExtractor::new();

        let content = "See [this note](my-note.md) and [that](notes/that.md)";
        let links = extractor.extract_links(content);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target, "my-note.md");
        assert_eq!(links[0].display_text.as_deref(), Some("this note"));
        assert!(matches!(links[0].link_type, LinkType::Markdown));
        assert_eq!(links[1].target, "notes/that.md");
    }

    #[test]
    fn test_ignore_external_urls() {
        let extractor = LinkExtractor::new();

        let content = "Visit [Google](https://google.com) or [email](mailto:test@test.com)";
        let links = extractor.extract_links(content);

        assert!(links.is_empty());
    }

    #[test]
    fn test_resolve_relative_path() {
        let extractor = LinkExtractor::new();

        let content = "[[Linked Note]]";
        let source = PathBuf::from("/notes/subfolder/my-note.md");
        let mut links = extractor.extract_and_resolve(content, &source);

        assert_eq!(links.len(), 1);
        let resolved = links[0].resolved_path.as_ref().unwrap();
        assert!(resolved.ends_with("Linked Note"));
        assert!(resolved.ends_with(".md") || resolved.to_string_lossy().contains("Linked Note"));
    }

    #[test]
    fn test_resolve_absolute_path() {
        let extractor = LinkExtractor::new();

        let content = "[Home](../index.md)";
        let source = PathBuf::from("/notes/level1/note.md");
        let mut links = extractor.extract_and_resolve(content, &source);

        assert_eq!(links.len(), 1);
        assert!(links[0].resolved_path.is_some());
    }

    #[test]
    fn test_extract_title_from_path() {
        assert_eq!(
            extract_title_from_path(Path::new("/notes/My-Note.md")),
            "My Note"
        );
        assert_eq!(
            extract_title_from_path(Path::new("/notes/my_note.md")),
            "my note"
        );
        assert_eq!(
            extract_title_from_path(Path::new("/notes/NoExtension")),
            "NoExtension"
        );
    }
}
