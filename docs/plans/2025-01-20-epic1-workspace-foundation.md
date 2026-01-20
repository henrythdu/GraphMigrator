# Epic 1: Workspace & Foundation

**Date**: 2025-01-20
**Status**: Ready for Implementation
**Phase**: Phase 1 (MVP)

## Goal

Set up the minimal two-crate workspace that compiles and runs, with placeholder structures for the graph system.

## Scope

- Create cargo workspace
- Set up `crates/cli` and `crates/core` with proper dependencies
- Define basic graph types (Node, Edge, Graph)
- Empty CLI that prints "GraphMigrator v0.1.0"
- Project builds successfully with `cargo build`

## Tasks

### 1.1 Initialize Workspace

- Create root `Cargo.toml` with workspace definition
- Add workspace dependencies (petgraph, serde, anyhow)
- Create `crates/` directory structure

### 1.2 Create Core Crate

- `crates/core/Cargo.toml` with dependencies
- `crates/core/src/lib.rs` with module declarations
- `crates/core/src/graph.rs` with basic types:
  - `pub struct Node` (id, name, node_type, language placeholder)
  - `pub struct Edge` (from, to, edge_type placeholder)
  - `pub struct Graph` (wrapper around petgraph::StableGraph)
- Empty module files: `parser/mod.rs`, `queries.rs`

### 1.3 Create CLI Crate

- `crates/cli/Cargo.toml` depending on `graph-migrator-core` and `clap` with derive feature
- `crates/cli/src/main.rs` using `clap` derive macro (`#[derive(Parser)]`)
- Use `#[command(version = "0.1.0")]` to handle `--version` automatically
- This establishes the CLI pattern for future commands (no refactoring needed later)

### 1.4 Verify Build

- Run `cargo build` — must succeed
- Run `cargo test` — should pass (even with no tests yet)
- Run `cargo run -- --version` — prints version

## Acceptance Criteria

- [ ] Workspace compiles without errors
- [ ] `cargo run` prints "GraphMigrator v0.1.0"
- [ ] Basic graph types compile in core crate
- [ ] CLI can import and use core types (even if just placeholder)

## Next Epic (Theme Only)

**Epic 2: Python Parsing via Tree-sitter**

*Goal*: Integrate tree-sitter to parse Python source code and construct an in-memory graph from a single file.

*Rough scope*: Add tree-sitter dependency, parse Python syntax tree, extract functions/classes as nodes, build basic graph structure.

*Note*: Evaluate parser design after implementing second language—refactor to trait-based if enum becomes unwieldy.
