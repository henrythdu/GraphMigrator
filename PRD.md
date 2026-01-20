PRD: GraphMigrator
## Visual Task-Tracking System for Code Migration

**Target Scope**: Large-scale code migrations (1000+ functions) with 2+ developers. This is a personal tool built to solve a real coordination problem during multi-language rewrites.

1. Executive Summary

GraphMigrator is a visual task-tracking system for managing large-scale code migrations. It transforms codebases into queryable dependency graphs, enabling human developers and AI agents to collaborate on language rewrites with full context awareness.

**The Core Problem: Coordination at Scale**

When migrating large codebases, the challenge isn't tracking individual tasks—it's understanding how those tasks interact. A spreadsheet can tell you "function X needs migration," but it can't answer:

- "If I change this Python function, which 15 other functions across 4 files will be affected?"
- "What's blocking me from deleting this legacy code?"
- "Which cross-language interfaces have become stale and need cleanup?"

GraphMigrator solves the **coordination problem** by making dependency relationships visible and queryable, letting developers and AI agents see the full impact of migration decisions before making them.

### Why Spreadsheets Fail at Scale

For small projects (dozens of functions), spreadsheets work fine. But at scale, they break down:

| **Aspect** | **Spreadsheet** | **GraphMigrator** |
|------------|-----------------|-------------------|
| **Dependency visibility** | Manual tracking, error-prone | Automatic: see all upstream/downstream dependencies |
| **Impact analysis** | "grep" and hope you caught everything | Query: "What breaks if I change this?" |
| **Cross-language edges** | No visibility into temporary interfaces | Track FFI boundaries and staleness |
| **AI context** | AI must scan entire codebase | AI queries graph for relevant nodes only |
| **Progress tracking** | Manual status updates | Bead integration: open task = work remains |
| **Multi-developer** | Merge conflicts, stale data | Real-time graph, consistent state |

**The Tipping Point**: Around 1000 functions and 2+ developers, manual coordination becomes a bottleneck. Each change requires checking spreadsheets, grepping code, and hoping nothing was missed. GraphMigrator automates the dependency tracking, freeing developers to focus on the migration itself.

1.1 Core Philosophy

GraphMigrator is NOT:
- An automated migration tool that rewrites code for you
- A build system or CI/CD tool
- A code generator

GraphMigrator IS:
- A command center for visualizing and tracking migration work
- A context provider for AI agents (upstream/downstream dependencies at a glance)
- A task tracker that links code symbols to persistent work items (Beads)
- A knowledge base for understanding codebase structure and dependencies

2. Project Structure (Workspace)

Following the "Zed" pattern, the project is organized into modular crates:

    crates/engine: The core library. Handles Tree-sitter parsing, LSP integration, graph management, and Git/Bead ID mapping.

    crates/server: An Axum web server that serves the graph as a JSON API and handles metadata updates.

    crates/cli: The primary entry point for the developer to initialize scans, sync with Beads, and launch the dashboard.

    frontend/: A React/D3.js dashboard for interactive visualization.

3. Key Functional Requirements

3.1 Graph Model

Nodes represent code elements at multiple granularities:
- Files (first-class nodes for structural context)
- Modules/Packages (for namespace organization)
- Classes/Interfaces/Structs (type definitions)
- Functions/Methods (executable code)
- Global Variables (shared state)

Edges represent relationships:
- contains: File → Class → Method (structural hierarchy)
- calls: Function → Function (execution dependencies)
- imports: Module/File → Module/File (code dependencies)
- inherits: Class → Class (OOP relationships)
- migrated_to: Legacy node → Target node(s) (migration links)

Node properties include:
- id: Stable, language-agnostic identifier
- name: Symbol name
- language: Source language (immutable)
- file_path: Location in codebase
- migration_status: pending | in_progress | migrated | superseded
- git_hash: Last analyzed commit (for staleness detection)
- bead_id: Links to persistent task context
- epic_bead_id: Links to parent epic/work group
- dependencies: External libraries/packages used
- suggested_alternatives: Target language equivalents
- purpose: Semantic description (AI-generated or manual)
- migration_reason: Free-text field explaining why this node was chosen for migration (e.g., "Performance bottleneck in hot path", "Security-critical auth logic", "Team wants to standardize on Rust")

3.2 Hybrid Analysis (Tree-sitter + LSP)

    Tree-sitter: Fast, language-agnostic parsing for initial graph construction. Extracts symbols, calls, and imports without requiring language servers or compilation.

    LSP Integration: Semantic enrichment for type-aware analysis. Provides accurate cross-file references, type information, and resolve-usages capabilities. Falls back to Tree-sitter-only mode if LSP unavailable.

3.3 Migration Tracking

    Non-Destructive: Legacy nodes are preserved with migration_status: "superseded". No 1-to-1 mapping assumption — one Python function may map to multiple Rust structs/functions.

    Cross-Language Edges: When migrated code calls legacy code, edges are marked with:
    - mechanism: pyo3 | http_rest | grpc | message_queue
    - temporary: true/false
    - created_at: Timestamp
    - staleness_indicator: green (<7 days) | yellow (7-30 days) | red (>30 days)

3.4 Task Integration (Beads)

    Flexible Assignment: Nodes can be assigned individual Bead IDs or grouped under epic Bead IDs. Developer or AI can cluster related functions under a single task.

    Context Linking: Each node links to its Bead, providing AI agents with full conversation history and task context when working on migration.

3.5 Visualization & Filtering

    Force-Directed Graph: D3.js-based interactive visualization showing nodes and edges with color coding:
    - 🔴 Red: Legacy/pending
    - 🟡 Yellow: In progress
    - 🟢 Green: Migrated
    - ⚪ Gray: Superseded

    Edge Styling:
    - Solid line: Same-language call
    - Dashed line: Cross-language call (color-coded by mechanism)
    - Dotted line: migrated_to link

    Filters: Show/hide by status, language, migration target, dependency type, etc.

    Dashboard Metrics: Progress percentage, cross-language edge count, stale edge warnings, estimated completion.

4. Technical Stack

    Language: Rust (Workspace architecture)
    Parsing: tree-sitter (primary), LSP (semantic enrichment)
    Graph Storage: petgraph (StableGraph for consistent IDs)
    Web Server: axum + tokio (Async I/O)
    Data Format: serde_json (State persistence)
    Frontend: React + D3.js (Visualization)
    Task Tracking: Beads (Persistent agent memory)

5. Typical Workflow

    Initialize: migrator init scans the codebase and builds the initial graph with all nodes and edges.

    Analyze: Developer explores the graph to understand structure, identifies migration candidates using filters (leaf nodes, CPU-intensive, security-critical, etc.).

    Plan: Developer or AI groups related nodes into migration waves and assigns Bead IDs for tracking.

    Execute: For each Bead/task:
        - AI agent queries the graph for context (dependencies, usages, related nodes)
        - Developer (with AI assistance) rewrites the code in target language
        - Update node status: legacy → in_progress → migrated
        - Create cross-language edges if needed (temporary during migration)

    Complete: As migration progresses, cross-language edges are replaced with same-language edges. Legacy nodes marked as "superseded" but preserved for reference.

    Visualize: Developer watches progress in real-time dashboard, identifies remaining work and stale cross-language dependencies.
