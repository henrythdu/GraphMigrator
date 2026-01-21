# Epic 7: Import Resolution and Cross-File Edges

**Date**: 2025-01-21
**Status**: Theme Only
**Phase**: Phase 1 (MVP)

## Goal

Resolve imports to actual file paths, create `EdgeType::Imports` edges in the graph, and enable cross-file call resolution using a symbol index. This completes the multi-file dependency graph.

> **Positioning Statement**: This is the final epic in the multi-file parsing series. It brings together Epic 4 (discovery), Epic 5 (merging), and Epic 6 (extraction) to answer the PRD's flagship question: "Which functions across files will be affected if I change this?"

## Scope

### What Epic 7 Does

- Resolve `ImportStatement` structs to actual file paths
- Create `EdgeType::Imports` edges between modules
- Build symbol index for cross-file lookups: `HashMap<(ModulePath, SymbolName), NodeId>`
- Resolve cross-file function calls using the symbol index
- Distinguish internal vs external dependencies

### What Epic 7 Does NOT Do (YAGNI)

- **No relative import resolution** (document as limitation)
- **No dynamic import handling** (document as limitation)
- **No BFS/DFS impact query layer** (add when needed)
- **No visualization** (future epic)

## Architecture (High-Level)

**Import Resolution**:
```rust
pub struct ImportResolver {
    project_root: PathBuf,
    stdlib_modules: HashSet<String>,
}

impl ImportResolver {
    pub fn resolve(&self, import: &ImportStatement) -> ResolutionStatus {
        // Check stdlib → External
        // Check file paths → Internal
        // Not found → Unresolved
    }
}

pub enum ResolutionStatus {
    Internal(PathBuf),
    External(String),
    Unresolved(String),
}
```

**Symbol Index**:
```rust
type SymbolIndex = HashMap<(ModulePath, SymbolName), NodeIndex>;

// Build during parsing:
symbol_index.insert(("myapp.utils", "helper"), node_idx);
```

**Cross-File Call Resolution**:
```rust
// During Pass 2, for each call:
if let Some(&target_idx) = symbol_index.get(&(module_path, call_name)) {
    // Found in another file → create cross-file edge
    graph.add_edge(caller_idx, target_idx, Edge { edge_type: EdgeType::Calls });
}
```

## Implementation Notes

**Two-pass architecture**:
1. Pass 1: Parse all files, build symbol index
2. Pass 2: Resolve imports, create edges, resolve cross-file calls

**Stdlib handling**:
- Maintain a set of known stdlib modules
- Don't create edges to external dependencies

**Known limitations to document**:
- Relative imports (level > 0) not resolved
- Dynamic imports not handled
- Namespace packages not supported

## Acceptance Criteria

Epic 7 is complete when:

- [ ] Imports resolve to internal file paths
- [ ] `EdgeType::Imports` edges are created between modules
- [ ] Cross-file calls are resolved using symbol index
- [ ] `EdgeType::Calls` edges connect nodes across files
- [ ] External dependencies are filtered out
- [ ] Integration test demonstrates cross-file dependency graph
- [ ] All Epic 3-6 tests still pass

## What Success Looks Like

After Epic 7, we can answer the PRD's flagship question:

```rust
let graph = parse_directory(Path::new("my_project"))?;

// Query: "What calls `helper()` across all files?"
let callers: Vec<_> = graph.edge_endpoints()
    .filter(|(_, to, edge)| {
        edge.edge_type == EdgeType::Calls
            && graph.node_weight(*to).map(|n| n.name == "helper") == Some(true)
    })
    .collect();

// Returns: callers from main.py, models.py, etc.
```

**This enables the PRD's core value proposition: multi-file impact analysis.**

## Next Epic (Theme Only)

**Epic 8: Impact Queries and CLI Commands**

*Goal*: Add CLI commands and query helpers for impact analysis.

*Theme*: Commands like `migrator impact <function-name>` to show upstream/downstream dependencies. Basic graph traversal helpers (no visualization yet).
