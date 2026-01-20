# Epic 3: Calls Edges (Single File)

**Date**: 2025-01-20
**Status**: Ready for Implementation
**Phase**: Phase 1 (MVP)

## Goal

Add call relationship edges between functions within a single Python file, enabling the graph to answer "which functions call which" for impact analysis. This transforms the graph from isolated symbols to a **relationship-aware** dependency structure.

> **Positioning Statement**: This epic validates the call-edge extraction mechanism as an MVP spike. It delivers intra-file dependency tracking, but **full PRD capability requires Epic 4** (multi-file parsing + import edges) to answer the PRD's flagship question: "which 15 other functions across 4 files will be affected?"

## Scope

### What Epic 3 Does

- Extract `call` nodes from the tree-sitter AST to identify where functions invoke other functions
- Create `Edge` with `EdgeType::Calls` between caller and callee function nodes
- Resolve call targets to function names already in the graph (e.g., `hello_world()` → node named "hello_world")
- Return a Graph with both nodes (from Epic 2) AND calls edges
- Add `edges()` iterator method to Graph for testing and querying

### What Epic 3 Does NOT Do (YAGNI)

- **No cross-file call resolution** - only edges within the same file
- **No import name resolution** - if `func()` calls `imported_module.other_func()`, we may not resolve the target (that's Epic 4)
- **No method calls** - Epic 2 doesn't extract methods, so we only handle top-level function calls
- **No LSP integration** - calls are "best-effort" syntax-based extraction
- **No edge visualization or querying infrastructure** - just the data structure

### Scope Constraints

- **Single file only** (inherits Epic 2's limitation)
- **Top-level functions only** (no nested function calls)
- **Simple name matching** (e.g., `call target name == graph node name`)

## Architecture

### Parser Extension Design

Epic 3 extends the Python parser from Epic 2. The key change is adding edge extraction alongside node extraction:

```rust
// crates/core/src/parser/python.rs

pub fn parse_file(path: &Path) -> anyhow::Result<Graph> {
    // 1. Canonicalize path
    let canonical_path = std::fs::canonicalize(path)?;

    // 2. Read file contents
    let source = std::fs::read_to_string(&canonical_path)?;

    // 3. Create tree-sitter parser and parse
    let mut parser = TsParser::new();
    parser.set_language(&LANGUAGE.into())?;
    let tree = parser.parse(&source, None)?;

    // 4. Extract top-level nodes (from Epic 2)
    let root_node = tree.root_node();
    let source_bytes = source.as_bytes();
    let nodes = extract_top_level_nodes(&root_node, &canonical_path, source_bytes);

    // 5. Build graph with nodes
    let mut graph = Graph::new();
    for node in nodes {
        graph.add_node(node);
    }

    // NEW: 6. Extract and add calls edges
    let edges = extract_calls_edges(&root_node, &canonical_path, source_bytes, &graph);
    for (from, to, edge) in edges {
        graph.add_edge(from, to, edge);
    }

    Ok(graph)
}
```

### Call Extraction Algorithm

The `extract_calls_edges` function walks the AST to find `call` nodes:

1. Walk the tree recursively (or use tree-sitter's query API for pattern matching)
2. For each `call` node, extract the function name being called
3. Find the caller function node (the parent function_definition containing this call)
4. Find the callee function node (lookup the target name in the graph)
5. Create an `Edge { edge_type: EdgeType::Calls }`

### Tree-sitter AST Structure for Calls

```python
# Source code
def caller():
    hello_world()  # This is a call node
```

Tree-sitter represents this as:
```
function_definition  (caller)
  └── block
        └── expression_statement
              └── call
                    ├── identifier: "hello_world"  # Target name
                    └── arguments (empty)
```

We extract the identifier name and match it against node names in the graph.

### Name Resolution Strategy

**File-scoped resolution**: `(file_path, name)` tuple matching

Since Epic 2 creates unique node IDs using `format!("{}::{}", file_path.display(), name)`, we use the same `(file_path, name)` tuple for lookups. This prevents false positives from duplicate function names across different files.

```rust
// Build lookup map after adding nodes
let mut node_map: HashMap<(PathBuf, String), NodeIndex> = HashMap::new();
for node in nodes {
    let idx = graph.add_node(node);
    // Use (file_path, name) as key for file-scoped resolution
    node_map.insert((node.file_path.clone(), node.name.clone()), idx);
}

// During edge extraction, resolve callee within same file only
if let Some(&callee_idx) = node_map.get(&(canonical_path.to_path_buf(), callee_name)) {
    // Create edge from caller_idx to callee_idx
}
```

This approach still has limitations:
- `imported_module.func()` - we won't resolve "imported_module.func" to a node (dotted name)
- `obj.method()` - we'll try to find "obj.method" as a function name (fails for methods)
- Aliased imports - `from x import y as z` then `z()` - we won't find "alias" in graph

These are acceptable for Epic 3's "best-effort" scope. Cross-file and import-aware resolution comes in Epic 4.

### Graph API Addition

Add an iterator method for testing. Tests need to verify edge endpoints (from, to) as well as edge type:

```rust
// crates/core/src/graph.rs

impl Graph {
    // ... existing methods ...

    /// Iterate over all edge weights in the graph
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.inner.edge_weights()
    }

    /// Get edge endpoints for testing verification
    /// Returns (from_node_index, to_node_index, edge_weight) for all edges
    pub fn edge_endpoints(&self) -> impl Iterator<Item = (NodeIndex, NodeIndex, &Edge)> {
        self.inner.all_edges()
            .map(|e| (e.source(), e.target(), self.inner.edge_weight(e).unwrap()))
    }
}
```

> **Note**: The `edge_endpoints()` method enables tests to assert `(from, to, edge_type)` tuples, not just edge weights. This is critical for verifying correct edge wiring.

## Limitations and Edge Cases

Epic 3's call extraction has known limitations. These are **acceptable** for the MVP scope but must be documented for future improvement:

### 1. Import-Aware Calls Not Resolved

**Problem**: Calls to imported functions can't be resolved to graph nodes.
- `import module; module.func()` → We extract "module.func" but won't find matching node
- `from module import func; func()` → Works! "func" matches node name
- `from module import func as alias; alias()` → Won't find "alias" in graph

**Impact**: Cross-module calls appear as unresolved or missing edges

**Future**: Epic 4's import edges + multi-file parsing enables cross-file resolution

### 2. Method Calls Not Supported

**Problem**: Method calls on objects can't be resolved.
- `obj.method()` → We'll try to find "obj.method" as a top-level function (fails)
- Epic 2 doesn't extract class methods, so we can't create edges to them

**Impact**: Method calls within classes are invisible to the graph

**Future**: When Epic X extracts methods, we'll add Class → Method edges and method call resolution

### 3. Dynamic Call Patterns Not Handled

**Problem**: Dynamically computed call targets can't be resolved.
- `func = getattr(module, "name"); func()` → Can't resolve dynamically computed call targets
- `dispatch[cmd]()` → Dictionary lookup, not a direct identifier

**Impact**: Some calls are missed entirely

**Future**: LSP integration provides semantic analysis for dynamic patterns

### 4. Nested Functions Not Considered

**Problem**: Nested functions aren't extracted as nodes (Epic 2 constraint).
- `def outer(): def inner(): inner()` → We don't extract "inner" as a node
- If we encounter `inner()` in `outer`, we can't resolve it

**Impact**: Nested function calls create dangling edges

**Future**: Add nested function extraction (if justified by use case)

### 5. Single-File Boundary

**Problem**: Can't create edges to functions defined in other files.
- `main.py` calls `utils.helper()` → We can't create edge to "helper" (different file)

**Impact**: Cross-file call graph is incomplete

**Future**: Epic 4's multi-file parsing enables cross-file edges

### 6. Ambiguous Call Sites

**Problem**: Multiple functions with same name (Python allows this in different scopes).
- Our simple name matching picks the first match or fails

**Impact**: Edge may connect to wrong target or not be created

**Future**: Scope-aware resolution (needs symbol table)

### Error Handling Strategy

- **Unresolved calls**: Log/skip, don't fail parsing. The graph is still useful with partial edges.
- **Multiple candidates**: Pick first match, log warning
- **No caller context**: Skip (e.g., top-level module-level call outside any function)

## Testing

### Test Fixtures

Create two test files to validate call extraction:

#### 1. `tests/test-fixtures/calls.py` - Simple Intra-File Calls

```python
def helper():
    pass

def caller():
    helper()  # Should create: caller → helper

def another_caller():
    helper()  # Should create: another_caller → helper

def isolated():
    pass  # No calls, no edges from this node
```

**Expected graph**: 4 nodes, 2 edges (caller→helper, another_caller→helper)

#### 2. `tests/test-fixtures/calls_with_unresolved.py` - Mixed Resolved/Unresolved

```python
import os
from sys import exit

def my_func():
    os.path.exists("file")  # Unresolved: "os.path.exists" not in graph
    exit(0)  # Unresolved: "exit" not defined in this file
    helper()

def helper():
    pass
```

**Expected graph**: 2 nodes (my_func, helper), 1 edge (my_func→helper). Unresolved calls are silently skipped.

#### 3. `tests/test-fixtures/calls_edge_cases.py` - Ambiguous and Edge Cases

```python
# Duplicate function names (should not create false edges within same file)
def helper():
    pass

def caller():
    helper()  # Resolves to first helper

def helper():  # Redeclaration (Python allows this)
    pass

# Dotted call (unresolved - dotted name not in graph)
def dotted_caller():
    os.path.exists("file")  # Won't resolve "os.path.exists"

# Method call (unresolved - methods not extracted)
def method_caller():
    obj = object()
    obj.method()  # Won't resolve "obj.method"
```

**Expected behavior**:
- Only 1 edge (caller→first helper) - name matching picks first match
- `os.path.exists()` skipped (dotted name)
- `obj.method()` skipped (method call)
- No crash on duplicate function names

> **Edge Yield Expectation**: Python codebases with significant method calls, imports, and dotted names may show **sparse edge coverage** initially. This is expected and acceptable for Epic 3's validation scope. Epic 4 will significantly improve edge yield through import-aware resolution.

### Unit Tests

```rust
#[test]
fn test_extract_calls_edges() {
    let parser = Parser::new();
    let graph = parser.parse_file(
        Path::new("tests/test-fixtures/calls.py"),
        &Language::Python,
    ).unwrap();

    // Should have 4 nodes from Epic 2
    assert_eq!(graph.node_count(), 4);

    // Should have 2 calls edges (caller→helper, another_caller→helper)
    assert_eq!(graph.edge_count(), 2);

    // Verify edge types
    for edge in graph.edges() {
        assert_eq!(edge.edge_type, EdgeType::Calls);
    }
}

#[test]
fn test_unresolved_calls_skipped() {
    // Verify that unresolved calls don't crash parsing
    // and that only resolvable edges are created
    let parser = Parser::new();
    let graph = parser.parse_file(
        Path::new("tests/test-fixtures/calls_with_unresolved.py"),
        &Language::Python,
    ).unwrap();

    // Should have 2 nodes
    assert_eq!(graph.node_count(), 2);

    // Should have 1 edge (my_func→helper), unresolved calls skipped
    assert_eq!(graph.edge_count(), 1);
}

#[test]
fn test_no_calls_no_edges() {
    // Verify that functions without calls create no edges
    let parser = Parser::new();
    let graph = parser.parse_file(
        Path::new("tests/test-fixtures/sample.py"),
        &Language::Python,
    ).unwrap();

    // sample.py has no function calls
    assert_eq!(graph.edge_count(), 0);
}
```

### Integration with Epic 2 Tests

Existing Epic 2 tests should still pass:
- `test_parse_python_file` → Now verifies nodes + no edges (sample.py has no calls)
- `test_nested_symbols_not_extracted` → Unchanged (Epic 3 doesn't extract nested)

## Implementation Notes

### Tree-Sitter Query API (Optional Enhancement)

Tree-sitter provides a query API that can simplify pattern matching. Instead of manual tree walking, we can use:

```rust
use tree_sitter::Query;

// Pattern to match call nodes
let query = Query::new(
    &LANGUAGE,
    "(call function: (identifier) @func-name)",
)?;

// Execute query and process matches
let mut query_cursor = QueryCursor::new();
for m in query_cursor.matches(&query, root_node, source_bytes) {
    // Extract @func-name captures
}
```

This is more efficient than manual traversal but adds a dependency on learning the query syntax. For Epic 3's simple scope, manual tree walking is sufficient. The query API can be evaluated for Epic 4 if needed.

### Node Index Tracking

To create edges, we need `NodeIndex` references (not just node names). Options:

1. **Build name → index map** after adding nodes:
   ```rust
   let mut node_map: HashMap<String, NodeIndex> = HashMap::new();
   for node in nodes {
       let idx = graph.add_node(node);
       node_map.insert(node.name.clone(), idx);
   }
   ```

2. **Pass node map to edge extraction** for lookup by name.

Option 1 is cleaner and keeps edge extraction independent of node addition order.

## Acceptance Criteria

Epic 3 is complete when:

- [ ] `parse_file()` returns a Graph with both nodes AND edges
- [ ] Calls edges are created for intra-file function calls (e.g., `caller()` calls `helper()` creates caller→helper edge)
- [ ] `EdgeType::Calls` enum variant exists and is used
- [ ] Unit test passes: `calls.py` fixture produces 2 edges for the 2 call sites
- [ ] Unresolved calls are handled gracefully (no panic/abort)
- [ ] All Epic 2 tests still pass (backward compatibility)
- [ ] Graph has `edges()` iterator method for testing
- [ ] Code is reviewed against PRD, Architecture, and this plan

## What Success Looks Like

After Epic 3, we can run:

```rust
let graph = parser.parse_file(Path::new("tests/test-fixtures/calls.py"), &Language::Python)?;

// Verify graph has edges
assert_eq!(graph.edge_count(), 2);

// Query: "Who calls helper?" using edge endpoints
let callers: Vec<&str> = graph.edge_endpoints()
    .filter(|(from, to, edge)| {
        edge.edge_type == EdgeType::Calls
            && graph.node_weight(*to).map(|n| n.name.as_str()) == Some("helper")
    })
    .map(|(from, _, _)| graph.node_weight(from).unwrap().name.as_str())
    .collect();

// Returns: ["caller", "another_caller"]
```

This validates the core coordination infrastructure. The graph now represents **relationships**, not just isolated symbols.

## Next Epic (Theme Only)

**Epic 4: Multi-File Parsing and Import Edges**

*Goal*: Parse multiple Python files into a single unified graph, adding import statement edges to enable cross-file dependency tracking.

*Rough scope*:
- Add `parse_directory()` or `parse_files()` function that accepts multiple file paths
- Implement `Graph::merge()` to combine nodes/edges from multiple files
- Extract `import_statement` and `import_from_statement` nodes from tree-sitter AST
- Create `EdgeType::Imports` edges between files/modules
- Enable cross-file call resolution by tracking which symbols are defined in which files
- Handle Python's import syntax: `import x`, `from x import y`, `import x as z`

*Note*: Evaluate whether to add "contains-lite" edges (File→Function/Class) for improved navigation without requiring method extraction.
