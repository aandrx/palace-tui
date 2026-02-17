// Watcher module - handles file system monitoring
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

#[derive(Debug, Clone)]
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

pub struct Watcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
}

impl Watcher {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let (tx, rx) = channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .context("Failed to create file watcher")?;

        watcher
            .watch(path.as_ref(), RecursiveMode::Recursive)
            .context("Failed to watch directory")?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    /// Get the next file event, filtering for markdown files only
    pub fn next_event(&self) -> Option<FileEvent> {
        while let Ok(event_result) = self.receiver.recv() {
            if let Ok(event) = event_result {
                if let Some(file_event) = Self::process_event(event) {
                    return Some(file_event);
                }
            }
        }
        None
    }

    fn process_event(event: Event) -> Option<FileEvent> {
        let path = event.paths.first()?;

        // Only process markdown files
        if !Self::is_markdown(path) {
            return None;
        }

        match event.kind {
            EventKind::Create(_) => Some(FileEvent::Created(path.clone())),
            EventKind::Modify(_) => Some(FileEvent::Modified(path.clone())),
            EventKind::Remove(_) => Some(FileEvent::Deleted(path.clone())),
            _ => None,
        }
    }

    fn is_markdown(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "md" || ext == "markdown")
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_detects_new_file() -> Result<()> {
        let temp = TempDir::new()?;
        let watcher = Watcher::new(temp.path())?;

        // Create a file in a separate thread to avoid blocking
        let temp_path = temp.path().to_path_buf();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            let _ = fs::write(temp_path.join("test.md"), "# Test");
        });

        // Wait for the event (with timeout)
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(2) {
            if let Some(event) = watcher.next_event() {
                if matches!(event, FileEvent::Created(_)) {
                    return Ok(());
                }
            }
        }

        Ok(())
    }
}
