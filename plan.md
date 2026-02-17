# Palace TUI: Personal Knowledge Graph Builder

## Project Vision
A terminal-based personal knowledge graph builder that automatically extracts entities and relationships from your notes, lets you explore connections visually, and helps you discover insights you didn't know you had. All data stays local, all processing happens on your machine.

---

## Phase 0: Foundation & Research (Weeks 1-2)

### Core Design Decisions
- **Language:** Rust (performance, safety, excellent TUI ecosystem)
- **Storage:** SQLite for metadata + file system for notes (Markdown)
- **Graph Engine:** petgraph for in-memory operations
- **TUI Framework:** ratatui (most active, feature-rich)

### Initial Setup
```bash
# Project structure
palace-tui/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── tui/              # TUI modules
│   ├── graph/            # Graph operations
│   ├── nlp/              # Entity extraction
│   ├── storage/          # Database and file handling
│   └── watcher/          # File system monitoring
├── tests/
├── docs/
└── examples/
```

### Key Dependencies to Evaluate

- **NLP:** rust-bert (comprehensive) vs finalfusion (lighter) vs linfa (ML toolkit)
- **Markdown parsing:** pulldown-cmark + comrak for extensions
- **File watching:** notify (cross-platform)
- **Graph visualization:** ratatui canvas + custom drawing
- **Search:** tantivy for full-text + vector search

---

## Phase 1: Core Infrastructure (Weeks 3-5)

### 1.1 Note Ingestion Engine

```rust

// Core traits and types
trait NoteSource {
    fn scan(&self) -> Vec<Note>;
    fn watch(&self) -> WatchStream;
}

struct Note {
    path: PathBuf,
    content: String,
    metadata: NoteMetadata,
    embeddings: Option<Vec<f32>>,
}
```

Features to implement:

- Recursive directory scanning
- Markdown file parsing with frontmatter support
- File system watcher for real-time updates
- Ignore patterns (.gitignore style)
- Incremental indexing (only changed files)

### 1.2 Storage Layer

```rust

struct Database {
    conn: sqlite::Connection,
}

// Tables
// - notes (id, path, modified, checksum)
// - entities (id, name, type, embedding)
// - relationships (source_id, target_id, relationship_type, strength)
// - note_entities (note_id, entity_id, position)
```

Features to implement:

- Schema migrations
- CRUD operations for notes and entities
- Full-text search indexes
- Performance optimization (indices, connection pooling)

### 1.3 Basic TUI Skeleton

```rust

struct App {
    notes: Vec<Note>,
    graph: Graph,
    selected_node: Option<NodeId>,
    mode: AppMode,  // Browse | Graph | Search | Settings
    input: String,
}

impl App {
    fn draw(&self, f: &mut Frame) {
        // Layout: main area + status bar
        // Navigation: tabs for different views
    }
}
```

Features to implement:

- Basic layout (header, main panel, footer)
- Keyboard navigation
- File browser view
- Simple note preview
- Status bar with mode indicator

---

## Phase 2: Entity Extraction (Weeks 6-8)

### 2.1 NLP Pipeline

```rust

struct EntityExtractor {
    model: Box<dyn NERModel>,
    cache: LruCache<String, Vec<Entity>>,
}

enum EntityType {
    Person,
    Organization,
    Concept,
    Project,
    Technology,
    Location,
    Date,
    Custom(String),
}

struct Entity {
    name: String,
    entity_type: EntityType,
    confidence: f32,
    span: (usize, usize),  // Position in text
    embedding: Option<Vec<f32>>,
}
```

Implementation plan:

- Start with rule-based extraction (regex, dictionaries)
- Add a small transformer model (distilled, quantized)
- Implement caching to avoid re-processing
- Allow manual entity tagging/override

### 2.2 Relationship Detection

```rust

struct Relationship {
    source: EntityId,
    target: EntityId,
    relationship_type: String,  // "works_on", "part_of", "mentions"
    strength: f32,  // Based on proximity, frequency, explicit links
    evidence: Vec<NoteId>,  // Where this relationship was found
}
```

Detection methods:

- Co-occurrence: Entities mentioned in same paragraph/section
- Syntactic patterns: "X works at Y", "X is a Y"
- Coreference resolution: "John...he" linking
- Explicit links: [[wiki-style links]] in notes

### 2.3 Embedding Generation

```rust

trait Embedder {
    fn embed_text(&self, text: &str) -> Result<Vec<f32>>;
    fn embed_entity(&self, entity: &Entity) -> Result<Vec<f32>>;
}

// Use small, fast models
// - all-MiniLM-L6-v2 (384 dim, ~80MB)
// - Or custom trained on technical docs
```

Features:

- Generate embeddings for notes and entities
- Store in SQLite for similarity search
- Batch processing for efficiency

---

## Phase 3: Graph Engine (Weeks 9-11)

### 3.1 Graph Data Structure

```rust

struct KnowledgeGraph {
    graph: StableGraph<Entity, Relationship>,
    note_map: HashMap<NoteId, Vec<NodeId>>,
    entity_index: HashMap<String, NodeId>,
}

impl KnowledgeGraph {
    fn add_note(&mut self, note: &Note, entities: Vec<Entity>) -> Vec<NodeId>;
    fn find_connections(&self, entity: &str, depth: usize) -> Vec<Path>;
    fn suggest_links(&self) -> Vec<PotentialLink>;
}
```

### 3.2 Link Suggestion Engine

```rust

struct LinkSuggestion {
    source: Entity,
    target: Entity,
    reason: String,  // "Similar embeddings", "Common context", etc.
    confidence: f32,
}
```

Algorithms:

- Similarity search: Find entities with close embeddings
- Graph traversal: Find missing links in common paths
- Pattern mining: "Entities that appear with X often also appear with Y"
- Temporal analysis: Entities that appear together over time

### 3.3 Query Language (Simple DSL)

```rust

// Examples
graph query "people working on database"
graph query "projects related to encryption since 2023"
graph neighbors "Rust" --depth 2 --type technology
```

Features:

- Natural language parsing (using local LLM or rules)
- Structured queries (like Cypher-lite)
- Result filtering and sorting

---

## Phase 4: TUI Development (Weeks 12-15)

### 4.1 Graph Visualization

```rust

struct GraphRenderer {
    layout: LayoutEngine,  // Force-directed, hierarchical, etc.
    zoom: f32,
    offset: (i32, i32),
    selected: Option<NodeId>,
}

impl GraphRenderer {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        // Use braille characters (⣀⣤⣶⣿) for smooth curves
        // Color nodes by type
        // Show edges with varying thickness based on strength
    }
}
```

Views:

- Force-directed layout: Organic exploration
- Radial layout: Focus on one node
- Timeline view: Show evolution over time
- Cluster view: Auto-detect communities

### 4.2 Interactive Features

```rust

enum Command {
    Select(usize),
    Expand(NodeId),  // Show more connections
    Focus(NodeId),   // Center on node
    Filter(String),  // Filter by type/tag
    Search(String),
    Edit(NoteId),    // Open note in $EDITOR
}
```

Interactions:

- Arrow keys to navigate nodes
- Enter to expand/collapse
- Tab to switch panels
- / to search
- e to edit source note
- i to inspect entity details

### 4.3 Search Panel

```rust

struct SearchEngine {
    fulltext: tantivy::Searcher,
    vector: VectorIndex,
    results: Vec<SearchResult>,
}

enum SearchResult {
    Note(Note),
    Entity(Entity),
    Relationship((Entity, Entity)),
}
```

Search types:

- Full-text search in notes
- Semantic search (nearest embeddings)
- Entity search by type/name
- Combined queries

---

## Phase 5: Advanced Features (Weeks 16-20)

### 5.1 Insight Discovery

```rust

struct Insight {
    title: String,
    description: String,
    evidence: Vec<NoteId>,
    confidence: f32,
    timestamp: DateTime,
}
```

Automatic insights:

- Emerging topics: Entities that are appearing more frequently
- Bridging concepts: Entities that connect disparate clusters
- Knowledge gaps: Topics mentioned but not well-connected
- Contradictions: Conflicting statements about same entity

### 5.2 Timeline View

```rust

struct Timeline {
    events: Vec<Event>,
    zoom_level: TimeScale,  // Day, Week, Month, Year
}

struct Event {
    timestamp: DateTime,
    entities: Vec<EntityId>,
    note: NoteId,
    event_type: EventType,  // Creation, Mention, Connection
}
```

Features:

- See when concepts first appeared
- Track how relationships evolved
- Identify periods of intense activity

### 5.3 Export & Integration

```rust

trait Exporter {
    fn export_graph(&self, format: ExportFormat) -> String;
    fn export_notes(&self, query: &str) -> Vec<Note>;
    fn generate_site(&self) -> Result<()>;  // Static site generator
}

enum ExportFormat {
    GraphML,
    JSON,
    CSV,
    Obsidian,  // Generate [[links]] back to original notes
}
```

Export targets:

- GraphML for Gephi visualization
- JSON for web apps
- Markdown with backlinks for Obsidian
- Static HTML site for sharing

### 5.4 Plugin System

```rust

trait PalaceTuiPlugin {
    fn name(&self) -> &str;
    fn initialize(&mut self, graph: &KnowledgeGraph) -> Result<()>;
    fn on_note_added(&mut self, note: &Note);
    fn on_query(&mut self, query: &str) -> Option<Vec<Insight>>;
}
```

Plugin examples:

- Zotero integration: Import academic papers and citations
- Git integration: Track changes over time
- Calendar sync: Connect notes to events
- Custom ML models: Bring your own entity extractor

---

## Phase 6: Polish & Distribution (Weeks 21-24)

### 6.1 Performance Optimization

- Profile with perf and flamegraph
- Optimize database queries
- Implement lazy loading for large graphs
- Add progress bars for long operations

### 6.2 Error Handling & Recovery

- Graceful degradation when models fail
- Automatic backup of database
- Crash recovery with transaction logs
- User-friendly error messages

### 6.3 Documentation

```markdown

## Documentation sections:
- Installation (cargo install, pre-built binaries)
- Quick start guide (5-minute setup)
- Configuration file reference
- Keybindings cheat sheet
- Plugin development guide
- FAQ and troubleshooting
```

### 6.4 Distribution

- Package managers: Homebrew, apt, cargo, winget
- Pre-built binaries: GitHub releases for all platforms
- Docker image: For server deployments
- Website: Project landing page with screenshots

---

## Future Possibilities (Post v1.0)

### Mobile Companion App

- Sync graph to phone
- Quick capture of ideas
- Voice notes with transcription
- QR code sharing between devices

### Multi-User Collaboration

- Shared knowledge graphs
- Merge changes from team members
- Permission management
- Real-time updates

### Advanced ML Features

- Fine-tuned models for technical domains
- Automatic summarization of note clusters
- Question answering over your knowledge base
- Trend prediction and forecasting

### IDE Integration

- VSCode extension to see connections while coding
- Emacs plugin for org-mode integration
- Vim plugin for quick entity lookup

---

## Getting Started Checklist

    Set up Rust development environment

    Create GitHub repository with MIT license

    Write basic README with vision statement

    Implement Phase 1.1 (Note ingestion)

    Implement Phase 1.2 (Storage layer)

    Implement Phase 1.3 (Basic TUI)

    Release v0.1.0-alpha (proof of concept)

    Gather feedback from early users

    Continue through remaining phases

Why This Project Matters

- **Privacy-first:** All processing happens locally, no cloud dependencies
- **Terminal-native:** Stays in your workflow, no context switching
- **Passive insight generation:** Discovers connections you didn't explicitly create
- **Extensible:** Plugin system allows infinite customization
- **Educational:** Great project for learning Rust, NLP, and graph theory

---

## Success Metrics

- **User adoption:** 500+ GitHub stars, 50+ daily users
- **Community contributions:** 10+ plugins from external developers
- **Performance:** Index 10,000 notes in under 5 minutes
- **Accuracy:** >80% precision/recall on entity extraction for technical docs
- **Stability:** 99.9% uptime (no crashes) for typical usage

---

## Technical Challenges & Solutions

| Challenge | Solution |
|-----------|----------|
| Large graphs >100k nodes | LevelDB for persistence, lazy loading |
| Slow ML models | Quantization, ONNX runtime, GPU support |
| Terminal rendering performance | Double-buffering, dirty region tracking |
| Cross-platform file watching | notify crate with fallback polling |
| SQLite contention | WAL mode, connection pooling |
