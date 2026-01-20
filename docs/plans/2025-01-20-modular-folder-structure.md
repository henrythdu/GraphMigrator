# Modular Folder Structure Design

**Date**: 2025-01-20
**Status**: Revised (Two-Phase Approach)
**Author**: Generated from brainstorming session, refined via PAL analysis

## Overview

This document describes a **two-phase approach** to folder structure for GraphMigrator:

1. **Phase 1 (Milestone 0)**: Minimal two-crate workspace for MVP validation
2. **Phase 2 (Optional)**: Modular architecture if MVP succeeds and pain points emerge

**Key Insight**: The PRD explicitly defines Milestone 0 as a "single-crate CLI" for validation. The original modular design ignored this directive and jumped straight to a complex architecture. This revised approach aligns with the PRD: start simple, refactor when needed.

---

## Phase 1: MVP - Minimal Two-Crate Workspace

**Goal**: Validate the core hypothesis with minimal overhead.

### Structure

```
GraphMigrator/
├── Cargo.toml              # Workspace definition
├── crates/
│   ├── cli/                # CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs      # CLI entry point only
│   └── core/                # Core library
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs       # Public API
│           ├── graph.rs     # Graph types + petgraph wrapper
│           ├── parser/
│           │   ├── mod.rs
│           │   └── python.rs # One language (tree-sitter only)
│           └── queries.rs   # Graph query functions
├── tests/
│   └── integration_test.rs
└── test-fixtures/
    └── sample-repo/
```

### Workspace Configuration

**Root `Cargo.toml`:**
```toml
[workspace]
members = [
    "crates/cli",
    "crates/core",
]

[workspace.dependencies]
# Shared versions for all crates
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
petgraph = "0.6"
anyhow = "1.0"
```

**CLI `Cargo.toml`:**
```toml
[package]
name = "graph-migrator-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
graph-migrator-core = { path = "../core" }
clap = { version = "4.4", features = ["derive"] }
```

**Core `Cargo.toml`:**
```toml
[package]
name = "graph-migrator-core"
version = "0.1.0"
edition = "2021"

[dependencies]
petgraph = "0.6"
serde = { version = "1.0", features = ["derive"] }
tree-sitter = "0.20"
tree-sitter-python = "0.19"
```

### Parser Design (Enum-Based, No Traits)

**File: `crates/core/src/parser/mod.rs`:**
```rust
mod python;

use crate::graph::Graph;
use std::path::Path;

pub enum Language {
    Python,
    // Add Rust, Go, etc. here later
}

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse_file(&self, path: &Path, lang: &Language) -> anyhow::Result<Graph> {
        match lang {
            Language::Python => {
                let content = std::fs::read_to_string(path)?;
                python::parse_source(&content)
            }
        }
    }
}
```

This approach:
- Avoids generics, traits, and dynamic dispatch
- Keeps code extremely simple and direct
- Makes adding the second language a one-line change in the match statement
- Follows YAGNI principle

### Benefits of Phase 1 Structure

| **Aspect** | **Why This Works** |
|------------|-------------------|
| **Enforced separation** | `core` never depends on `cli` (compiler enforced) |
| **Isolated testing** | `cargo test -p graph-migrator-core` tests only core logic |
| **Minimal overhead** | 2 crates vs 1, negligible complexity |
| **Idiomatic Rust** | Standard pattern for CLI + library |
| **Fast to build** | No feature flags, no trait complexity |
| **Future-proof** | Adding `crates/server` later is natural extension |

### When to Move to Phase 2

Extract to modular architecture ONLY when hitting specific pain points:

| **Trigger** | **Action** |
|-------------|------------|
| Adding second language | Extract `parser/` to its own module (still within core) |
| Third language needed | Consider `parsers/` crate with feature flags |
| Need persistent storage | Add `storage.rs` module, then crate if complex |
| Need test isolation | Add `memory_storage()` function before trait abstraction |
| Code becomes unwieldy | Extract crate ONLY when module is too large |

**Principle**: Refactor from simplicity to modularity, not from complexity to more complexity.

---

## Phase 2: Modular Architecture (Optional, Future)

**Only implement if MVP validates the concept AND specific pain points emerge.**

This section preserves the original modular design as a reference architecture for potential future evolution.

### Overview

GraphMigrator can evolve into a **trait-based, feature-flagged architecture** that enables:
- Adding new languages without touching core code
- Swapping storage backends via feature flags
- Optional UI/API components (CLI can run standalone)
- Independent parser development and versioning

### Structure (Reference Architecture)

```
GraphMigrator/
├── crates/
│   ├── core/              # Trait definitions + shared types
│   ├── engine/            # Graph engine (uses traits from core)
│   ├── parsers/           # Parser implementations
│   │   ├── interface/     # Parser trait only
│   │   ├── tree-sitter/   # Shared utilities
│   │   ├── lsp/           # Shared LSP client
│   │   └── langs/         # Language-specific crates
│   ├── storage/           # Storage backends
│   │   ├── interface/     # Storage trait only
│   │   ├── json/          # JSON file storage
│   │   └── memory/        # In-memory storage
│   ├── api/               # Server (optional feature)
│   └── cli/               # CLI binary
├── frontend/              # React dashboard (optional feature)
└── tests/
```

### Parser System

**Parser Trait:**
```rust
pub trait Parser {
    fn parse_file(&self, path: &Path) -> Result<Vec<Node>>;
    fn get_symbols(&self, node: &Node) -> Result<Vec<Symbol>>;
    fn get_dependencies(&self, node: &Node) -> Result<Vec<Dependency>>;
}
```

**Adding a New Language:**
1. Create `crates/parsers/langs/go/`
2. Implement `Parser` trait
3. Add feature flag: `go = ["parsers/langs/go"]`

### Storage System

**Storage Trait:**
```rust
pub trait Storage: Send + Sync {
    fn load_graph(&self) -> Result<MigrationGraph>;
    fn save_graph(&self, graph: &MigrationGraph) -> Result<()>;
    fn node_exists(&self, id: &str) -> Result<bool>;
    fn get_node(&self, id: &str) -> Result<Option<Node>>;
    fn update_node(&self, node: &Node) -> Result<()>;
}
```

**Feature Flags:**
```toml
[features]
default = ["json-storage"]
json-storage = ["storage/json"]
memory-storage = ["storage/memory"]
```

### Feature Flags (Full Stack)

```toml
[features]
default = ["python", "rust", "json-storage"]

# Languages
python = ["parsers/langs/python"]
rust = ["parsers/langs/rust"]
javascript = ["parsers/langs/javascript"]
all-langs = ["python", "rust", "javascript"]

# Storage backends
json-storage = ["storage/json"]
memory-storage = ["storage/memory"]

# Optional components
api = ["dep:api", "dep:axum"]
frontend = ["api"]

# Developer convenience
dev = ["all-langs", "json-storage", "memory-storage"]
```

---

## Migration Path: Phase 1 → Phase 2

### Example: Adding Second Language

**Phase 1 (Current):**
```rust
// crates/core/src/parser/mod.rs
pub enum Language {
    Python,
    Rust,  // ← Add this
}
```

**Phase 1+ (Growing):**
```rust
// Still no traits needed, just more match arms
pub enum Language {
    Python,
    Rust,
    JavaScript,
    Go,
}
```

**Trigger for Phase 2:**
- When `parser/` module becomes >500 lines
- When languages need different tree-sitter versions
- When you want to publish parsers as separate crates

### Example: Adding Storage Backend

**Phase 1:**
```rust
// crates/core/src/storage.rs
pub fn save_graph(graph: &Graph, path: &Path) -> anyhow::Result<()> {
    // JSON implementation
}
```

**Phase 1+ (Growing):**
```rust
pub enum StorageBackend {
    Json(PathBuf),
    Memory,
}

impl StorageBackend {
    pub fn save(&self, graph: &Graph) -> anyhow::Result<()> {
        match self {
            Self::Json(path) => { /* ... */ }
            Self::Memory => { /* ... */ }
        }
    }
}
```

**Trigger for Phase 2:**
- When you need third backend
- When backends have divergent dependencies
- When you want to test storage independently

---

## Design Philosophy

### Make It Work, Make It Right, Make It Fast

1. **Make it work** (Phase 1): Build the simplest thing that validates the hypothesis
2. **Make it right** (Phase 2): Refactor into modularity when pain points emerge
3. **Make it fast** (Later): Optimize compile times, runtime performance

### Anti-Patterns to Avoid

❌ **"Boil the ocean"** - Building full architecture before validating concept
❌ **"Premature abstraction"** - Traits for 2 implementations
❌ **"Speculative flexibility"** - Feature flags for static configuration
❌ **"Commercial-scale thinking"** - Optimizing for 10+ languages in a personal project

### Principles to Follow

✅ **YAGNI** - You Ain't Gonna Need It (until you actually do)
✅ **Refactor when needed** - Pain points drive architecture, not speculation
✅ **Two crates is enough** - `cli` + `core` covers most MVP needs
✅ **Enums over traits** - Simpler for small, fixed sets of variants
✅ **Modules over crates** - Extract to crate only when module is unwieldy

---

## Testing Strategy

### Phase 1 Testing

```
tests/
├── integration_test.rs     # Core functionality tests
└── test-fixtures/
    └── sample-repo/        # Small Python codebase
```

**Run tests:**
```bash
# Test core library only
cargo test -p graph-migrator-core

# Run all tests
cargo test
```

### Phase 2 Testing (When Needed)

```
tests/
├── integration/
│   ├── python_basic_test.rs
│   ├── rust_basic_test.rs
│   └── multi_lang_test.rs
├── fixtures/
│   ├── python_repo/
│   ├── rust_repo/
│   └── mixed_repo/
└── Cargo.toml
```

---

## Next Steps

### Immediate (Phase 1)

1. Create workspace: `cargo new`
2. Create `crates/core` and `crates/cli`
3. Implement basic graph structures in `core/src/graph.rs`
4. Implement Python parser in `core/src/parser/python.rs`
5. Build CLI with one command: `migrator find-dependencies <function>`
6. Test against sample repository

### Future (Phase 2 - Only If Needed)

1. Add second language → evaluate if parser module needs extraction
2. Add persistent storage → evaluate if storage trait is needed
3. Add API server → create `crates/api` crate
4. Add frontend → create `frontend/` directory

**Key**: Each step is triggered by actual need, not architectural speculation.

---

## Analysis Notes

This document was revised based on multi-tool analysis (PAL Consensus → Challenge → ThinkDeep) that identified:

**Key Finding**: PRD Milestone 0 specifies "single-crate CLI" but original folder structure described a 9+ crate modular architecture. This was a fundamental mismatch.

**Challenges Validated**:
- Over-engineering for personal project scope (3-4 languages)
- Unnecessary trait abstractions for static configurations
- Feature flags for rarely-changing builds
- Optimizing for commercial scale (10+ languages) in a personal tool

**Expert Refinement**: Minimal two-crate workspace (`cli` + `core`) provides better structure than single binary crate while maintaining simplicity.

**Recommendation**: Start with Phase 1, evolve to Phase 2 only when hitting specific pain points.
