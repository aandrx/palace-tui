// Storage module - handles database and file system operations
mod database;
mod ingester;
mod links;
mod notes;
mod scanner;

pub use database::{Database, NoteLink, NoteLinkStats};
pub use ingester::{Ingester, IngestionStats};
pub use links::{LinkExtractor, LinkType, ParsedLink};
pub use notes::{Note, NoteMetadata};
pub use scanner::{NoteScanner, ScanConfig};
