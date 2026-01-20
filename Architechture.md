# Architecture Design Document: GraphMigrator

## 1. System Overview

GraphMigrator is a visual task-tracking system for code migration. It transforms codebases into queryable dependency graphs, enabling human developers and AI agents to collaborate on language rewrites with full context awareness. The system provides real-time visualization of migration progress and links code symbols to persistent task tracking via Beads.

### 1.1 Design Principles

- **Non-Destructive**: Legacy code is preserved, not deleted. Nodes transition through states: pending → in_progress → migrated → superseded
- **Language-Agnostic**: Supports any source/target language via Tree-sitter and LSP
- **Context-Rich**: Every node carries metadata (dependencies, purpose, suggested alternatives, migration reasons)
- **Developer-Driven**: AI assists but humans make migration decisions
- **Incremental**: Supports gradual migration with cross-language temporary interfaces
- **Flexible**: No hardcoded migration categories — developers provide their own reasons

## 2. Component Architecture

The system follows a decoupled workspace architecture:

```
GraphMigrator/
├── crates/
│   ├── engine/      # Core: parsing, graph management, LSP integration
│   ├── server/      # Axum REST API
│   └── cli/         # Command-line interface
├── frontend/        # React/D3.js dashboard
└── .migrator/       # Graph state persistence
```

### 2.1 Core Engine (crates/engine)

**Parser Module (Hybrid: Tree-sitter + LSP)**
- Tree-sitter integration for fast, language-agnostic parsing
- LSP client integration for semantic enrichment (type-aware analysis)
- Cross-language reference resolution
- Graceful degradation: Tree-sitter-only if LSP unavailable

**Graph Controller**
- Implemented using `petgraph::StableGraph` for consistent node IDs
- Supports multiple node types: File, Module, Class, Function, Global Variable
- Edge types: contains, calls, imports, inherits, migrated_to
- Cross-language edge tracking with mechanism metadata

**Metadata Enrichment**
- Dependency extraction (external libraries/packages)
- Purpose annotation (AI-generated semantic summaries)
- Suggested alternatives mapping (source lib → target lib)
- Impact metrics (inbound/outbound call counts)

**Persistence Layer**
- Atomic writes to `.migrator/state.json` using `serde_json`
- Git hash tracking for staleness detection
- Event log for historical analysis (optional)

### 2.2 API Server (crates/server)

**Protocol**: REST API powered by Axum and Tokio

**State Sharing**: `Arc<RwLock<MigrationGraph>>` for thread-safe concurrent access

**Endpoints**:
- `GET /graph` - Stream full graph or filtered view
- `GET /node/:id` - Get single node with full metadata
- `GET /node/:id/neighbors` - Get upstream/downstream dependencies
- `PATCH /node/:id` - Update migration status, Bead ID, metadata
- `POST /node/:id/alternatives` - Add/update suggested library alternatives
- `GET /metrics` - Progress stats, cross-language edge counts, stale warnings
- `GET /filters` - Available filter options (languages, statuses, migration reasons, etc.)

### 2.3 CLI (crates/cli)

**Commands**:
- `migrator init <repo-path>` - Initialize graph from codebase
- `migrator scan` - Re-scan for changes (update Git hashes)
- `migrator serve` - Start API server and dashboard
- `migrator enrich` - Trigger AI enrichment for semantic metadata
- `migrator status` - Show migration progress summary

### 2.4 Frontend (frontend/)

**Technology**: React + D3.js

**Components**:
- Force-directed graph visualization
- Node detail panel (metadata, dependencies, alternatives)
- Filters panel (status, language, migration reason, etc.)
- Dashboard metrics (progress, warnings)
- Bead integration (view/create/link tasks)

## 3. Data Model

### 3.1 Node Schema

```rust
pub struct Node {
    // Identity
    pub id: String,              // Stable, language-agnostic ID
    pub name: String,            // Symbol name
    pub qualified_name: String,  // Fully qualified path
    pub node_type: NodeType,     // File | Module | Class | Function | GlobalVar
    pub language: String,        // Immutable: source language

    // Location
    pub file_path: PathBuf,
    pub line_range: Option<(usize, usize)>,

    // Migration tracking
    pub migration_status: MigrationStatus,  // Pending | InProgress | Migrated | Superseded
    pub migration_target: Option<String>,   // Target language (e.g., "rust")
    pub migration_reason: Option<String>,   // Why migrate? (free-text)
    pub superseded_by: Vec<String>,         // IDs of replacement nodes (if any)
    pub supersedes: Vec<String>,            // IDs of legacy nodes (if any)

    // Task linking
    pub bead_id: Option<String>,            // Individual task
    pub epic_bead_id: Option<String>,       // Parent epic/work group

    // Code metadata
    pub dependencies: Vec<Dependency>,      // External libraries
    pub suggested_alternatives: Vec<Alternative>,  // Target language libs
    pub purpose: Option<String>,            // Semantic description

    // Impact metrics
    pub inbound_calls: usize,               // How many things call this
    pub outbound_calls: usize,              // How many things this calls
    pub estimated_effort: Option<String>,   // "2-3 days", etc.

    // Staleness tracking
    pub git_hash: String,                   // Last analyzed commit
    pub last_updated: DateTime<Utc>,
}

pub enum NodeType {
    File,
    Module,
    Class,
    Interface,
    Struct,
    Function,
    Method,
    GlobalVariable,
    MigrationUnit,   // Groups nodes for n-to-m migrations
}

/// MigrationUnit represents a logical grouping of code nodes being migrated together.
/// Used when one legacy node maps to multiple new nodes, or multiple legacy nodes
/// are consolidated into fewer new nodes.
pub struct MigrationUnit {
    pub id: String,
    pub name: String,                // e.g., "Auth Service Refactor"
    pub description: String,         // What this migration accomplishes
    pub status: MigrationStatus,     // Pending | InProgress | Completed
    pub legacy_nodes: Vec<String>,   // IDs of nodes being replaced
    pub target_nodes: Vec<String>,   // IDs of new nodes
    pub created_at: DateTime<Utc>,
    pub bead_id: Option<String>,     // Link to task context
}

pub enum MigrationStatus {
    Pending,        // Not yet migrated
    InProgress,     // Currently being worked on
    Migrated,       // Successfully migrated
    Superseded,     // Legacy node, replaced by new implementation
}

pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub language: String,
}

pub struct Alternative {
    pub name: String,
    pub language: String,
    pub package: String,
    pub confidence: f32,        // 0.0 - 1.0
    pub notes: String,
}
```

### 3.2 Edge Schema

```rust
pub struct Edge {
    pub id: String,
    pub from: String,           // Node ID
    pub to: String,             // Node ID
    pub edge_type: EdgeType,

    // For cross-language calls
    pub cross_language: Option<CrossLanguageInfo>,
}

pub enum EdgeType {
    Contains,       // File → Class → Method
    Calls,          // Function → Function
    Imports,        // Module → Module, File → File
    Inherits,       // Class → Class
    MigratedTo,     // Legacy → Target
    PartOfMigration, // Node → MigrationUnit (groups related migrations)
}

pub struct CrossLanguageInfo {
    pub mechanism: Mechanism,
    pub temporary: bool,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub blocking_beads: Vec<String>,    // Tasks blocking removal
}

pub enum Mechanism {
    PyO3,           // Python ↔ Rust FFI
    HttpRest,       // HTTP/REST API
    Grpc,           // gRPC
    MessageQueue,   // Kafka, RabbitMQ, etc.
    Other(String),   // Custom mechanism
}
```

### 3.3 Graph State

```rust
pub struct MigrationGraph {
    pub nodes: StableGraph<Node, Edge>,
    pub metadata: GraphMetadata,
    pub event_log: Vec<GraphEvent>,
}

pub struct GraphMetadata {
    pub repo_path: PathBuf,
    pub last_scan: DateTime<Utc>,
    pub git_commit: String,
    pub total_nodes: usize,
    pub migration_progress: f32,  // 0.0 - 1.0
}

pub enum GraphEvent {
    NodeCreated { id: String, actor: String },
    NodeUpdated { id: String, changes: Vec<String>, actor: String },
    EdgeCreated { id: String, actor: String },
    EdgeUpdated { id: String, actor: String },
    MigrationStatusChanged { id: String, from: MigrationStatus, to: MigrationStatus },
}
```

### 3.4 MigrationUnit: Handling n-to-m Migrations

When one legacy function becomes multiple new functions (or vice versa), direct `migrated_to` edges create navigation ambiguity. The `MigrationUnit` node solves this by grouping related migrations.

**Example: Python `backward()` → 3 Rust structures**

Without MigrationUnit:
```
python_value_backward --[migrated_to]--> rust_value_backward
python_value_backward --[migrated_to]--> rust_backward_graph
python_value_backward --[migrated_to]--> rust_backwardable_trait
```
Query: "What replaced `python_value_backward`?" Returns 3 nodes—but no semantic grouping.

With MigrationUnit:
```
python_value_backward --[part_of_migration]--> mu_backward_refactor
rust_value_backward --[part_of_migration]--> mu_backward_refactor
rust_backward_graph --[part_of_migration]--> mu_backward_refactor
rust_backwardable_trait --[part_of_migration]--> mu_backward_refactor
```
Query: "What replaced `python_value_backward`?" Returns `mu_backward_refactor`, which contains:
- `description`: "Refactored backward pass from closures to explicit graph + trait"
- `legacy_nodes`: ["python_value_backward"]
- `target_nodes`: ["rust_value_backward", "rust_backward_graph", "rust_backwardable_trait"]
- `status`: InProgress

**Benefits**:
- Clear semantic grouping of related transformations
- Single query point for "what's happening here?"
- Status tracking at the migration level (not individual node level)
- Bead linkage applies to the whole migration, not scattered nodes

## 4. Data Flow

### 4.1 Initialization Flow

```
1. migrator init <repo>
   ↓
2. Tree-sitter parses all files
   ↓
3. Graph built with raw nodes/edges
   ↓
4. LSP enrichment (if available)
   ↓
5. Optional: AI enrichment (purpose, alternatives)
   ↓
6. Graph persisted to .migrator/state.json
```

### 4.2 Migration Workflow Flow

```
1. Developer explores graph (filters, visualization)
   ↓
2. Identifies migration candidate(s)
   - Filter by inbound_calls (find leaf nodes)
   - Filter by migration_reason (group by rationale)
   ↓
3. Assigns Bead ID (individual or epic for group)
   ↓
4. AI agent queries graph for context:
   - GET /node/:id/neighbors (upstream/downstream)
   - Gets dependencies, suggested alternatives
   ↓
5. Developer rewrites code (with AI assistance)
   ↓
6. Updates node status: pending → in_progress
   ↓
7. When complete:
   - Create new target node(s)
   - Add migrated_to edge(s)
   - Mark legacy as superseded
   - Update any cross-language edges
   ↓
8. Repeat until migration complete
```

### 4.3 Staleness Detection Flow

```
1. migrator scan (or git hook)
   ↓
2. Compare current file Git hashes vs stored node git_hash
   ↓
3. If mismatch:
   - Re-parse changed file(s)
   - Update affected nodes/edges
   - Flag as stale if node changed since last analyzed
   ↓
4. Trigger re-enrichment if needed
```

### 4.4 Cross-Language Staleness Warning Flow

```
1. Periodic check (on dashboard load or via CLI)
   ↓
2. For each cross-language edge:
   - Calculate age: now - last_updated
   - If > 30 days: mark as RED (stale)
   - If 7-30 days: mark as YELLOW (warning)
   - If < 7 days: mark as GREEN (fresh)
   ↓
3. Dashboard shows warnings:
   - "⚠️ 12 cross-language edges are stale (>30 days)"
   - List affected edges with blocking_beads
   ↓
4. Developer can prioritize tasks that block cleanup
```

## 5. Scaling & Performance

**Concurrency**: Rust's ownership model via `Arc<RwLock<MigrationGraph>>` prevents data races during concurrent updates from multiple AI agents or UI connections.

**Frontend Rendering**:
- Canvas-based force graph (D3.js) for smooth interaction with 5,000+ nodes
- Virtual scrolling for node lists
- Lazy loading of node details
- Level-of-detail rendering (simplify distant nodes)

**Memory Efficiency**:
- Code snippets stored on disk, loaded on-demand
- Streaming API responses for large graphs
- Configurable graph subset queries

**Caching**:
- In-memory LRU cache for frequently accessed nodes
- Indexed queries by node type, language, status, migration_reason

## 6. Security & Privacy

**Local-First**: Graph and source code remain on local machine. Only semantic summaries sent to LLM providers (if enrichment enabled).

**Auditability**: Every graph change logged with timestamp and actor (Human or Agent ID).

**Access Control**: (Future) Authentication for multi-user environments.

## 7. Implementation Roadmap

**Milestone 0: MVP - Read-Only CLI (Validation Spike)**
*Goal: Test core hypothesis—Is a dependency graph more valuable than grep/spreadsheets?*

- Single-crate CLI application (`graph-migrator-cli`)
- Single language support (e.g., Python or Rust)
- In-memory graph (no persistence yet)
- Tree-sitter parsing for one language
- Read-only queries:
  - `migrator find-dependencies <function-name>` - Show upstream/downstream calls
  - `migrator find-leaf-nodes` - Functions with no dependents (good migration candidates)
  - `migrator visualize` - Simple ASCII graph output
- No server, no UI, no persistence, no LSP

**Success Criteria**: Can this CLI answer a question that grep cannot, with less effort? If yes, proceed to Milestone 1. If no, pivot.

---

**Milestone 1: Core Infrastructure**
- Rust workspace setup (engine, server, cli crates)
- Tree-sitter integration for 2-3 languages
- Basic graph construction (nodes + structural edges)
- CLI init and scan commands
- JSON persistence to `.migrator/state.json`
- Git hash tracking for staleness detection

**Testing Strategy**:
- Unit tests for graph construction (verify node/edge counts match known codebases)
- Integration tests with sample repositories (test against fixed codebases with known structures)
- Property-based tests for graph queries (verify properties like "no orphan nodes")
- Round-trip tests (parse → serialize → deserialize → verify equality)

**Milestone 2: API & Persistence**
- Axum server with basic endpoints
- JSON persistence to .migrator/state.json
- Git hash tracking and staleness detection

**Milestone 3: Frontend Visualization**
- React + D3.js force-directed graph
- Basic node detail panel
- Filters (status, language, migration_reason)

**Milestone 4: Rich Metadata**
- LSP integration for semantic enrichment
- Dependency extraction
- Suggested alternatives (manual initially, AI-assisted later)

**Milestone 5: Beads Integration**
- Link nodes to Beads (optional task tracking)
- Epic/task hierarchy
- Cross-language edge tracking
- MigrationUnit node support for n-to-m migrations

**Milestone 6: Advanced Features**
- Cross-language staleness warnings (time-based)
- Impact metrics (inbound/outbound call counts)
- Historical event log
- AI-assisted enrichment (purpose, alternatives, migration_reasons)

## 8. Example: Non-1-to-1 Migration

**Python (Legacy)**:
```python
# micrograd/value.py
class Value:
    def backward(self, grad=1.0):
        # Complex implementation with closures
        pass
```

**Rust (Target)**:
```rust
// Multiple new concepts
pub struct Value { /* ... */ }

impl Value {
    pub fn backward(&self, grad: f32) { /* ... */ }
}

// New structures that didn't exist in Python
pub struct BackwardGraph { /* ... */ }
pub trait Backwardable { /* ... */ }
```

**Graph Representation (with MigrationUnit)**:
```json
{
  "nodes": [
    {
      "id": "python_value_backward",
      "name": "backward",
      "language": "python",
      "node_type": "Method",
      "migration_status": "superseded"
    },
    {
      "id": "rust_value_backward",
      "name": "backward",
      "language": "rust",
      "node_type": "Method",
      "migration_status": "migrated"
    },
    {
      "id": "rust_backward_graph",
      "name": "BackwardGraph",
      "language": "rust",
      "node_type": "Struct",
      "migration_status": "migrated"
    },
    {
      "id": "mu_backward_refactor",
      "name": "Backward Pass Refactor",
      "node_type": "MigrationUnit",
      "description": "Refactored backward pass from Python closures to explicit Rust graph + trait",
      "status": "Completed",
      "legacy_nodes": ["python_value_backward"],
      "target_nodes": ["rust_value_backward", "rust_backward_graph", "rust_backwardable_trait"],
      "bead_id": "beads-123"
    }
  ],
  "edges": [
    {
      "from": "python_value_backward",
      "to": "mu_backward_refactor",
      "edge_type": "PartOfMigration"
    },
    {
      "from": "rust_value_backward",
      "to": "mu_backward_refactor",
      "edge_type": "PartOfMigration"
    },
    {
      "from": "rust_backward_graph",
      "to": "mu_backward_refactor",
      "edge_type": "PartOfMigration"
    }
  ]
}
```

**Query Example**:
```
GET /node/python_value_backward/neighbors

Response: {
  "migration_groups": [
    {
      "id": "mu_backward_refactor",
      "description": "Refactored backward pass...",
      "status": "Completed",
      "all_replacements": [
        "rust_value_backward",
        "rust_backward_graph",
        "rust_backwardable_trait"
      ]
    }
  ]
}
```

Single query returns the complete migration context, not scattered edges.
