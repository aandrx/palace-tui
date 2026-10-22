use crate::storage::Database;
use crate::tree::NoteTree;
use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Tree,
    Search,
    Help,
}

pub struct App {
    pub mode: AppMode,
    pub tree: NoteTree,
    pub selected_idx: usize,
    pub search_query: String,
    pub should_quit: bool,
    pub scroll_offset: usize,
}

impl App {
    pub fn new(db: &Database) -> Result<Self> {
        let paths = db.get_note_paths_with_links()?;
        let tree = NoteTree::from_paths(&paths);

        Ok(Self {
            mode: AppMode::Tree,
            tree,
            selected_idx: 0,
            search_query: String::new(),
            should_quit: false,
            scroll_offset: 0,
        })
    }

    pub fn handle_key_event(&mut self, event: Event) {
        match event {
            Event::Key(KeyEvent { code, .. }) => self.handle_key_code(code),
            Event::Resize(_, _) => {}
            _ => {}
        }
    }

    fn handle_key_code(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.mode == AppMode::Search {
                    self.mode = AppMode::Tree;
                    self.search_query.clear();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('?') => {
                if self.mode == AppMode::Help {
                    self.mode = AppMode::Tree;
                } else {
                    self.mode = AppMode::Help;
                }
            }
            KeyCode::Char('/') | KeyCode::Char('s') => {
                self.mode = AppMode::Search;
                self.search_query.clear();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max_idx = self.tree.total_notes.saturating_sub(1);
                if self.selected_idx < max_idx {
                    self.selected_idx += 1;
                }
            }
            KeyCode::Home => {
                self.selected_idx = 0;
            }
            KeyCode::End => {
                self.selected_idx = self.tree.total_notes.saturating_sub(1);
            }
            KeyCode::PageUp => {
                self.selected_idx = self.selected_idx.saturating_sub(10);
            }
            KeyCode::PageDown => {
                let max_idx = self.tree.total_notes.saturating_sub(1);
                self.selected_idx = (self.selected_idx + 10).min(max_idx);
            }
            _ => {}
        }

        if let KeyCode::Char(c) = code {
            if self.mode == AppMode::Search {
                self.search_query.push(c);
                self.search_by_query();
            }
        }
    }

    fn search_by_query(&mut self) {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            return;
        }

        let files = self.tree.get_all_files();
        for (idx, name, _, _) in &files {
            if name.to_lowercase().contains(&query) {
                self.selected_idx = *idx;
                break;
            }
        }
    }

    pub fn render(&mut self, width: usize, height: usize) -> Vec<String> {
        let tree_lines = self.tree.render(width, Some(self.selected_idx));
        let visible_lines = height.saturating_sub(2);

        if tree_lines.len() <= visible_lines {
            self.scroll_offset = 0;
            tree_lines
        } else {
            if self.selected_idx < self.scroll_offset {
                self.scroll_offset = self.selected_idx;
            } else if self.selected_idx >= self.scroll_offset + visible_lines {
                self.scroll_offset = self.selected_idx - visible_lines + 1;
            }

            let max_scroll = tree_lines.len().saturating_sub(visible_lines);
            self.scroll_offset = self.scroll_offset.min(max_scroll);

            tree_lines
                .iter()
                .skip(self.scroll_offset)
                .take(visible_lines)
                .cloned()
                .collect()
        }
    }

    pub fn get_status_bar(&self) -> String {
        let mode_str = match self.mode {
            AppMode::Tree => "File Tree",
            AppMode::Search => "Search",
            AppMode::Help => "Help",
        };

        let search_hint = if self.mode == AppMode::Search {
            format!(" | Search: {}_", self.search_query)
        } else {
            String::new()
        };

        format!(
            " [{}] | Notes: {} | Links: {} | Selected: {}/{}{}",
            mode_str,
            self.tree.total_notes,
            self.tree.total_links,
            self.selected_idx + 1,
            self.tree.total_notes,
            search_hint
        )
    }

    pub fn get_help_text(&self) -> Vec<String> {
        vec![
            "Palace TUI - Keyboard Shortcuts".to_string(),
            "".to_string(),
            "Navigation:".to_string(),
            "  ↑↓ or j/k     Move selection up/down".to_string(),
            "  PageUp/Dn     Move 10 items at a time".to_string(),
            "  Home/End      Jump to first/last item".to_string(),
            "".to_string(),
            "Actions:".to_string(),
            "  / or s        Search for notes".to_string(),
            "  ?             Toggle this help".to_string(),
            "  q or Esc      Quit (or exit search/help)".to_string(),
            "".to_string(),
            "Legend:".to_string(),
            "  📁 Folder     Directory containing notes".to_string(),
            "  📄 File       Markdown note file".to_string(),
            "  [N links]     Number of outgoing links".to_string(),
            "  ▸ Selected    Currently selected item".to_string(),
        ]
    }
}
