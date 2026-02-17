// Storage module - handles database and file system operations
mod database;
mod ingester;
mod notes;
mod scanner;

pub use database::Database;
pub use ingester::{Ingester, IngestionStats};
pub use notes::{Note, NoteMetadata};
pub use scanner::{NoteScanner, ScanConfig};
