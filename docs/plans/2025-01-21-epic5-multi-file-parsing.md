# Epic 5: Multi-File Parsing (Graph Merging)

**Date**: 2025-01-21
**Status**: Ready to Implement
**Phase**: Phase 1 (MVP)

## Goal

Parse multiple Python files and merge their individual graphs into a unified project graph with provenance metadata. This builds on Epic 4's file discovery and Epic 3's single-file parser.

> **Positioning Statement**: This epic focuses on parsing multiple files and merging graphs with provenance tracking. It does NOT handle import extraction or cross-file resolution—those are future epics.
>
> **Key Design Decision**: Following PAL consensus (Gemini 2.5 Pro, GPT-5.2, Claude Opus 4.5), Epic 5 implements **Pass 1 of a two-pass architecture**: collect symbols and locations (provenance metadata) without doing import resolution. This is foundational work, not scope creep.

## Scope

### What Epic 5 Does

- Use `discover_python_files()` from Epic 4 to get file list
- Parse each file using existing `parse_file()` from Epic 3
- Merge individual file graphs into a unified `Graph`
- Track provenance: which file each node came from
- Handle duplicate node IDs (first occurrence wins)
- Design API for Epic 7's two-pass resolution

### What Epic 5 Does NOT Do (YAGNI)

- **No import extraction** - that's Epic 6
- **No import resolution** - that's Epic 7
- **No cross-file call resolution** - that's Epic 7
- **No creating EdgeType::Imports edges** - that's Epic 7

### Scope Constraints

- Provenance metadata only (node → file mapping)
- Within-file edges preserved
- API designed for Epic 7 consumption
- Graph merging with index remapping

## Architecture

### Core Principle: Two-Pass Foundation

Epic 5 implements **Pass 1** of the multi-pass architecture:
- **Pass 1 (Epic 5)**: Parse all files, collect symbols, establish provenance
- **Pass 2 (Epic 7)**: Resolve imports, create cross-file edges

The key insight: "not doing import resolution" doesn't mean "not collecting the metadata needed for resolution." Provenance tracking (node → file mapping) is essential foundation, not optional complexity.

### Data Flow

```
[Epic 4: discover_python_files()]
         ↓
    Vec<PathBuf>  (file paths)
         ↓
[Epic 5: parse_files()]
         ↓
    Sort paths (deterministic order)
         ↓
    For each file:
      1. Call parse_file() from Epic 3
      2. Tag each node with source_file
      3. Add nodes to unified graph (deduplicate by ID)
      4. Add edges with remapped indices
         ↓
    MultiFileGraph {
        graph: Graph,                                  // Merged nodes + edges
        node_locations: HashMap<NodeId, PathBuf>,      // Provenance metadata
        file_nodes: HashMap<PathBuf, NodeIndex>,       // Reverse lookup
    }
```

### Data Structures

```rust
/// Result of parsing multiple files into a unified graph
///
/// This structure is designed as the foundation for Epic 7's two-pass
/// resolution. It contains the merged graph plus provenance metadata
/// needed for future cross-file edge creation.
pub struct MultiFileGraph {
    /// The unified graph containing all nodes and edges from parsed files
    pub graph: Graph,

    /// Maps each node to its source file path
    ///
    /// This is NOT import resolution - it's provenance metadata that
    /// Epic 7 will use when resolving cross-file dependencies.
    ///
    /// Purpose: Answer "where is this node defined?" for impact analysis
    pub node_locations: HashMap<NodeId, PathBuf>,

    /// Reverse lookup: maps file paths to their file-node indices
    ///
    /// Purpose: Enable efficient "what nodes are in this file?" queries
    /// and support Epic 7's import resolution by providing file-level
    /// entry points into the graph.
    pub file_nodes: HashMap<PathBuf, NodeIndex>,
}

impl MultiFileGraph {
    /// Create a new empty MultiFileGraph
    fn new() -> Self;

    /// Merge a single-file graph into this multi-file graph
    ///
    /// Handles node deduplication and edge index remapping
    fn merge_file_graph(&mut self, file_graph: Graph, source_file: &Path) -> anyhow::Result<()>;
}
```

### API Surface

```rust
// crates/core/src/parser/mod.rs

/// Parse multiple Python files into a unified multi-file graph
///
/// # Arguments
/// * `paths` - Slice of file paths to parse
///
/// # Returns
/// A `MultiFileGraph` containing the merged graph and provenance metadata
///
/// # Example
/// ```no_run
/// use graph_migrator_core::parser;
///
/// let files = vec![
///     Path::new("src/main.py"),
///     Path::new("src/utils.py"),
/// ];
///
/// let multi = parser::parse_files(&files)?;
/// println!("Parsed {} nodes from {} files",
///          multi.graph.node_count(),
///          multi.file_nodes.len());
/// ```
pub fn parse_files(paths: &[&Path]) -> anyhow::Result<MultiFileGraph>

/// Parse all Python files in a directory
///
/// This is a convenience wrapper that combines Epic 4's file discovery
/// with Epic 5's multi-file parsing.
///
/// # Arguments
/// * `root` - Root directory to search and parse
///
/// # Returns
/// A `MultiFileGraph` containing all Python files in the directory
///
/// # Example
/// ```no_run
/// use graph_migrator_core::parser;
///
/// let multi = parser::parse_directory(Path::new("my_project"))?;
/// println!("Parsed {} nodes", multi.graph.node_count());
/// ```
pub fn parse_directory(root: &Path) -> anyhow::Result<MultiFileGraph>
```

### Graph Merging Strategy

**Node deduplication**: Node IDs are globally unique with format `file_path::symbol_name` (verified in `crates/core/src/parser/python.rs:80`). This means:
- `file1.py::helper` and `file2.py::helper` have different IDs → both kept
- Same file parsed twice → same ID → first wins (correct deduplication)

**Edge remapping**: When adding edges, remap node indices from file graph to project graph using a HashMap.

**Deterministic merging**: Sort file paths before processing to ensure reproducible output across runs.

```rust
// Pseudocode for merge_file_graph()
fn merge_file_graph(&mut self, file_graph: Graph, source_file: &Path) -> Result<()> {
    let mut index_map: HashMap<NodeIndex, NodeIndex> = HashMap::new();

    // Add nodes (skip duplicates, track index mappings)
    for node_idx in file_graph.node_indices() {
        let node = &file_graph[node_idx];
        if let Some(existing_idx) = self.find_node_by_id(&node.id) {
            // Duplicate: use existing node
            index_map.insert(node_idx, existing_idx);
        } else {
            // New node: add to graph
            let new_idx = self.graph.add_node(node.clone());
            index_map.insert(node_idx, new_idx);

            // Track provenance
            self.node_locations.insert(node.id.clone(), source_file.to_path_buf());
        }
    }

    // Add edges (remap indices)
    for edge in file_graph.edge_indices() {
        let (source, target) = file_graph.edge_endpoints(edge).unwrap();
        let new_source = index_map[&source];
        let new_target = index_map[&target];
        let weight = file_graph[edge].clone();
        self.graph.add_edge(new_source, new_target, weight);
    }

    Ok(())
}
```

## Implementation Tasks

**Task 1: Add MultiFileGraph structure**
- [ ] Create `MultiFileGraph` struct in `parser/mod.rs`
- [ ] Add `graph`, `node_locations`, and `file_nodes` fields
- [ ] Implement `new()` constructor
- [ ] Add `HashMap` imports if needed
- [ ] Add doc comments explaining each field's purpose

**Task 2: Implement graph merging logic**
- [ ] Add `merge_file_graph()` method to `MultiFileGraph`
- [ ] Implement node deduplication by ID (first occurrence wins)
- [ ] Implement edge index remapping using HashMap tracking
- [ ] Populate `node_locations` and `file_nodes` during merge

**Task 3: Implement parse_files() function**
- [ ] Accept `&[&Path]` as input
- [ ] **Sort paths for deterministic merging**
- [ ] Iterate through paths, calling `parse_file()` for each
- [ ] Merge each file graph into unified `MultiFileGraph`
- [ ] Return `MultiFileGraph` with provenance metadata

**Task 4: Implement parse_directory() function**
- [ ] Call `discover_python_files()` from Epic 4
- [ ] Convert `Vec<PathBuf>` to `Vec<&Path>` for `parse_files()`
- [ ] Delegate to `parse_files()`

**Task 5: Write unit tests**
- [ ] Test single-file parsing (backward compatibility)
- [ ] Test multi-file parsing with 2+ files
- [ ] Test duplicate node ID handling (same file, same symbol)
- [ ] Test edge preservation during merge
- [ ] Test `node_locations` mapping
- [ ] Test `file_nodes` reverse lookup
- [ ] Test deterministic output (same files → same graph)

**Task 6: Integration test**
- [ ] Create multi-file test fixture (2-3 Python files)
- [ ] Verify nodes from all files are present
- [ ] Verify edges are preserved
- [ ] Verify provenance metadata is correct
- [ ] Test same-name functions in different files remain distinct

**Task 7: Documentation**
- [ ] Add doc comments to `MultiFileGraph`
- [ ] Add doc examples to `parse_files()` and `parse_directory()`
- [ ] Document the two-pass design intent
- [ ] Document NodeId format: `file_path::symbol_name`

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_parse_files_single_file() {
    let files = vec![Path::new("tests/fixtures/simple.py")];
    let multi = parse_files(&files).unwrap();

    assert_eq!(multi.file_nodes.len(), 1);
    assert!(multi.graph.node_count() > 0);
}

#[test]
fn test_parse_files_multiple_files() {
    let files = vec![
        Path::new("tests/fixtures/a.py"),
        Path::new("tests/fixtures/b.py"),
    ];
    let multi = parse_files(&files).unwrap();

    assert_eq!(multi.file_nodes.len(), 2);
    // Verify nodes from both files are present
}

#[test]
fn test_duplicate_node_handling() {
    // Same file parsed twice: same NodeId → first wins
}

#[test]
fn test_same_name_different_files() {
    // file1.py::helper and file2.py::helper have different IDs
    // Both should be present in the merged graph
}

#[test]
fn test_edge_preservation() {
    // Verify edges from file graphs are preserved
    // with correct indices in merged graph
}

#[test]
fn test_node_locations_mapping() {
    let multi = parse_files(&files).unwrap();

    // Verify node_locations maps each node to its file
    for (node_id, file_path) in &multi.node_locations {
        assert!(multi.file_nodes.contains_key(file_path));
    }
}

#[test]
fn test_deterministic_merging() {
    // Parse same files in different orders
    // Verify resulting graph is identical
}
```

### Integration Test Fixture

```
tests/test-fixtures/multi-file-project/
├── module_a.py
│   └── def helper(): ...
├── module_b.py
│   └── def process(): ... calls helper()
└── main.py
    └── def main(): ... calls process()
```

**Expected behavior:**
- All 3 files parsed
- 3 function nodes + 3 file nodes = 6 nodes minimum
- Edges within each file preserved
- `node_locations` correctly maps each function to its file
- `file_nodes` contains all 3 file paths
- `module_a.py::helper` ≠ `module_b.py::helper` (different NodeIds)

### Edge Cases

| Case | Behavior |
|------|----------|
| Empty file list | Returns empty `MultiFileGraph` |
| Non-existent file | Returns error (propagated from `parse_file()`) |
| Files with duplicate node IDs | First occurrence wins, subsequent skipped |
| Files with no symbols | File node added, no other nodes |
| Same name, different files | Both kept (different NodeIds: `file1.py::name` vs `file2.py::name`) |

## Acceptance Criteria

Epic 5 is complete when:

- [ ] `parse_files()` returns a `MultiFileGraph` from multiple files
- [ ] `parse_directory()` discovers and parses all `.py` files in a directory
- [ ] Duplicate node IDs are handled (first occurrence wins)
- [ ] Same-name functions in different files remain distinct
- [ ] Edges are preserved during graph merging
- [ ] `node_locations` correctly maps each node to its source file
- [ ] `file_nodes` provides reverse lookup from file to file-node index
- [ ] Merging is deterministic (same input → same output)
- [ ] All unit tests pass (single-file, multi-file, deduplication, edges, metadata)
- [ ] Integration test with multi-file fixture passes
- [ ] All Epic 3 and Epic 4 tests still pass (backward compatibility)
- [ ] API is documented with doc comments and examples
- [ ] NodeId format (`file_path::symbol_name`) is documented

## What Success Looks Like

After Epic 5, we can run:

```rust
use graph_migrator_core::parser;

// Parse all Python files in a project
let multi = parser::parse_directory(Path::new("my_project"))?;

println!("Parsed {} nodes from {} files",
         multi.graph.node_count(),
         multi.file_nodes.len());

// Query: what functions are in this file?
if let Some(file_node_idx) = multi.file_nodes.get(Path::new("src/utils.py")) {
    // Iterate edges from file node to find contained functions
}

// Query: where is this node defined?
if let Some(source_file) = multi.node_locations.get(&node_id) {
    println!("Node {} is in {:?}", node_id, source_file);
}

// Foundation for Epic 7: MultiFileGraph is ready for two-pass resolution
// Pass 1 (Epic 5): Collect symbols ✓
// Pass 2 (Epic 7): Resolve imports and create cross-file edges
```

## Dependencies

Add to `crates/core/src/parser/mod.rs`:
- `std::collections::HashMap` for provenance tracking

## Round 5 Design Review (2025-01-21)

### PAL Consensus Results

**Models Consulted**: Gemini 2.5 Pro (for), GPT-5.2 (against), Claude Opus 4.5 (neutral)
**Confidence**: 8/10 across all models

**Verdict**: ✅ **READY TO IMPLEMENT** with minor clarifications

| Aspect | Finding |
|--------|---------|
| Two-pass architecture | ✓ Correct - industry standard pattern |
| `MultiFileGraph` structure | ✓ Appropriate for scope |
| YAGNI compliance | ✓ No over-engineering detected |
| Node deduplication strategy | ✓ **VERIFIED SAFE** - see below |

### Critical Verification: NodeId Uniqueness

**Initial Concern**: Both Gemini and GPT-5.2 identified that "deduplicate by ID (first wins)" could incorrectly merge distinct functions.

**Code Inspection**: Checked actual NodeId generation in `crates/core/src/parser/python.rs:80`:
```rust
id: format!("{}::{}", file_path.display(), name)
```

**Conclusion**: NodeId format is `file_path::symbol_name`, which means:
- `file1.py::helper` and `file2.py::helper` have **different** IDs
- Both nodes are kept (not collapsed) ✓
- Deduplication only occurs for true duplicates (same file, same symbol) ✓

**Status**: The design is correct as written.

### Additional Recommendations

| Recommendation | Priority | Action |
|---------------|----------|--------|
| Deterministic merging | High | Sort paths before processing |
| Document file_nodes purpose | Medium | Add doc comments |
| Span-based provenance | Low | Defer to future (YAGNI) |

### PAL Challenge Result

The challenge tool asked whether `MultiFileGraph` is over-engineering. Result: Design upheld as appropriate scope.

## PAL Consensus Summary

**Date**: 2025-01-21 (Initial) + 2025-01-21 (Round 5 Review)
**Models Consulted**: Gemini 2.5 Pro (for), GPT-5.2 (against), Claude Opus 4.5 (neutral)

**Initial Consensus (8-9/10 confidence)**:
- Implement Option 2 (Pass 1 foundation) + minimal file tracking
- File tracking is "provenance metadata" not "import resolution"
- Simple aggregation (Option 1) creates ID collisions and technical debt
- Two-pass design needs stable IDs now to avoid Epic 7 breakage

**Round 5 Verification (8/10 confidence)**:
- NodeId format verified as `file_path::symbol_name` (globally unique)
- Deduplication strategy confirmed safe
- Design ready for implementation
- Add deterministic path sorting for reproducibility

**Key insight**: 1-2 hours of design now saves a day of refactoring in Epic 7.

## Next Epic (Already Documented)

**Epic 6: Import Statement Extraction**

*Goal*: Extract import statements from Python AST into structured data (Ruff-inspired).

*Theme*: Parse `import` and `from ... import` syntax, return `ImportStatement` structs. No resolution yet—just structured extraction.
