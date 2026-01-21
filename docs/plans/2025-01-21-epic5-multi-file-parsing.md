# Epic 5: Multi-File Parsing (Graph Merging)

**Date**: 2025-01-21
**Status**: Draft
**Phase**: Phase 1 (MVP)

## Goal

Parse multiple Python files and merge their individual graphs into a unified project graph. This builds on Epic 4's file discovery and Epic 3's single-file parser.

> **Positioning Statement**: This epic focuses on parsing multiple files and merging graphs. It does NOT yet handle import edges or cross-file resolution—those are future epics.

## Scope

### What Epic 5 Does

- Use `discover_python_files()` from Epic 4 to get file list
- Parse each file using existing `parse_file()` from Epic 3
- Merge individual file graphs into a unified `Graph`
- Handle duplicate node IDs (existing behavior: first occurrence wins)

### What Epic 5 Does NOT Do (YAGNI)

- **No import extraction** - future epic
- **No import resolution** - future epic
- **No cross-file call resolution** - future epic
- **No import edges** - future epic

## Architecture

### API Surface

```rust
/// Parse multiple Python files into a unified graph
pub fn parse_files(paths: &[&Path]) -> anyhow::Result<Graph> {
    // For each path:
    //   1. Call parse_file() from Epic 3
    //   2. Merge nodes (skip duplicates by ID)
    //   3. Merge edges (remap node indices)
}

/// Parse all Python files in a directory
pub fn parse_directory(root: &Path) -> anyhow::Result<Graph> {
    let files = crate::discovery::discover_python_files(root);
    parse_files(&files)
}
```

### Graph Merging Strategy

**Node deduplication**: Node IDs are unique (`file_path::name`), so we skip nodes that already exist.

**Edge remapping**: When adding edges, remap node indices from file graph to project graph using a HashMap.

```rust
// Pseudocode
for file in files {
    let file_graph = parse_file(file)?;

    // Add nodes (skip duplicates)
    for node in file_graph.nodes() {
        if !project_graph.has_node(&node.id) {
            project_graph.add_node(node);
        }
    }

    // Add edges (remap indices)
    for edge in file_graph.edges() {
        let source = project_graph.find_node(edge.source_id);
        let target = project_graph.find_node(edge.target_id);
        project_graph.add_edge(source, target, edge);
    }
}
```

## Implementation Tasks

1. **Add `parse_files()` function**
   - [ ] Accept `&[&Path]` as input
   - [ ] Iterate through paths, calling `parse_file()` for each
   - [ ] Track node ID mappings for edge remapping
   - [ ] Return unified `Graph`

2. **Add `parse_directory()` function**
   - [ ] Call `discover_python_files()` from Epic 4
   - [ ] Delegate to `parse_files()`

3. **Implement graph merging logic**
   - [ ] Skip duplicate nodes by ID
   - [ ] Remap node indices for edges
   - [ ] Preserve edge types

4. **Unit tests**
   - [ ] Test parsing single file
   - [ ] Test parsing multiple files
   - [ ] Test duplicate node handling
   - [ ] Test edge preservation

5. **Integration test**
   - [ ] Create multi-file fixture
   - [ ] Verify nodes from all files are present
   - [ ] Verify edges are preserved

## Acceptance Criteria

Epic 5 is complete when:

- [ ] `parse_files()` returns a unified Graph from multiple files
- [ ] `parse_directory()` discovers and parses all files in a directory
- [ ] Duplicate node IDs are handled (first occurrence wins)
- [ ] Edges are preserved during merging
- [ ] All unit tests pass
- [ ] Integration test passes
- [ ] All Epic 3 and Epic 4 tests still pass

## What Success Looks Like

After Epic 5:

```rust
let graph = parse_directory(Path::new("my_project"))?;

println!("Parsed {} nodes from multiple files", graph.node_count());
// Graph contains nodes from all files, edges within each file
```

## Next Epic (Theme Only)

**Epic 6: Import Statement Extraction**

*Goal*: Extract import statements from Python AST into structured data (Ruff-inspired).

*Theme*: Parse `import` and `from ... import` syntax, return `ImportStatement` structs. No resolution yet—just structured extraction.
