# Epic 2: Python Parsing via Tree-sitter

**Date**: 2025-01-20
**Status**: Ready for Implementation
**Phase**: Phase 1 (MVP)

## Goal

Validate that tree-sitter can parse Python code and extract basic symbols (functions, classes) into graph nodes. This proves the core technology works before investing in more complex features.

## Scope

**What Epic 2 Does:**
- Add `tree-sitter` and `tree-sitter-python` dependencies
- Create Python parser in `crates/core/src/parser/python.rs`
- Parse a single Python file and extract:
  - Function names → `Node` with `NodeType::Function`
  - Class names → `Node` with `NodeType::Class`
  - File path for location tracking
- Return a `Graph` containing these nodes

**What Epic 2 Does NOT Do (YAGNI):**
- No edge construction (calls, contains, imports)—that's Epic 3+
- No full Python syntax coverage (methods, decorators, etc.)—just top-level functions and classes
- No CLI integration—just a library function for now
- No multi-file parsing—single file only
- No line range tracking—deferred until actually needed

## Architecture

### Parser Module Structure

```
crates/core/src/parser/
├── mod.rs          # pub enum Language { Python }, pub struct Parser
└── python.rs       # pub fn parse_file(path: &Path) -> Result<Graph>
```

### Public API

```rust
// crates/core/src/parser/mod.rs
pub enum Language {
    Python,
}

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Parser {}
    }

    pub fn parse_file(&self, path: &Path, lang: &Language) -> anyhow::Result<Graph> {
        match lang {
            Language::Python => python::parse_file(path),
        }
    }
}
```

### Python Parser Implementation

```rust
// crates/core/src/parser/python.rs
use tree_sitter::{Parser as TsParser, Language};
use tree_sitter_python::LANGUAGE;
use crate::graph::{Graph, Node, NodeType};
use std::path::Path;

pub fn parse_file(path: &Path) -> anyhow::Result<Graph> {
    // 1. Canonicalize path for stable node IDs (prevents duplicate IDs from relative/absolute paths)
    let canonical_path = std::fs::canonicalize(path)?;

    // 2. Read file contents to String
    let source = std::fs::read_to_string(&canonical_path)?;

    // 3. Create tree-sitter parser
    let mut parser = TsParser::new();
    parser.set_language(&LANGUAGE.into())?;

    // 4. Parse source code (returns Result, not Option in tree-sitter 0.26+)
    let tree = parser.parse(&source, None)?;

    // 5. Extract nodes
    let root_node = tree.root_node();
    let mut functions = extract_functions(&root_node, &canonical_path);
    let classes = extract_classes(&root_node, &canonical_path);

    // 6. Build graph
    let mut graph = Graph::new();
    for node in functions.into_iter().chain(classes) {
        graph.add_node(node);
    }

    Ok(graph)
}

fn extract_functions(root_node: &tree_sitter::Node, file_path: &Path) -> Vec<Node> {
    // Walk tree, find function_definition nodes
    // Extract name, create Node with NodeType::Function
    todo!()
}

fn extract_classes(root_node: &tree_sitter::Node, file_path: &Path) -> Vec<Node> {
    // Walk tree, find class_definition nodes
    // Extract name, create Node with NodeType::Class
    todo!()
}
```

## Data Flow

```
1. User provides Python file path
   ↓
2. Canonicalize path (for stable node IDs)
   ↓
3. Read file contents to String
   ↓
4. Create tree-sitter Parser
   ↓
5. Parse source code into SyntaxTree
   ↓
6. Walk tree, extract functions and classes
   ↓
7. Create Graph with nodes
   ↓
8. Return Graph
```

## Node Structure

**ID Format:** `canonical_path::symbol_name` (e.g., `/abs/path/to/file.py::my_function`)

> **Important**: Paths are canonicalized using `std::fs::canonicalize()` before ID generation to ensure stable, reproducible identifiers regardless of whether the input path is relative or absolute.

```rust
Node {
    id: format!("{}::{}", canonical_path.display(), name),
    name: name.to_string(),
    node_type: NodeType::Function or NodeType::Class,
    language: "python".to_string(),
    file_path: canonical_path.to_path_buf(),
    line_range: None,  // Deferred - not needed for MVP validation
    // ... migration fields left as placeholder
}
```

## Dependencies

```toml
# crates/core/Cargo.toml
[dependencies]
tree-sitter = "0.26"
tree-sitter-python = "0.25"
```

> **Note**: Breaking changes from earlier tree-sitter versions (0.20 → 0.26):
> - `tree_sitter_python::language()` function replaced with `LANGUAGE` constant
> - `parser.set_language(&LANGUAGE.into())` required to load grammar
> - `parse()` returns `Result<Tree, Box<dyn Error>>` instead of `Option<Tree>`
>
> These changes were discovered during PAL ThinkDeep investigation (2025-01-20).

## Implementation Notes

### Path Canonicalization
All file paths are canonicalized using `std::fs::canonicalize()` before ID generation. This ensures stable, reproducible node identifiers regardless of whether the input path is relative or absolute. Without canonicalization, `./src/main.py` and `/app/src/main.py` would generate duplicate nodes for the same file.

### Future API Improvements (Optional)
The following suggestions were identified during PAL pre-commit validation for future consideration:
- **Simplify Parser API**: The `Parser` struct is currently stateless. Consider refactoring to a free function `parser::parse_file()` if no state is needed long-term.
- **Display trait for Language**: Implement `Display` for `Language` enum to derive language strings dynamically instead of hardcoding `"python"`.

These are **not** part of Epic 2 scope (YAGNI) but documented for future evaluation.

## Testing

### Test Fixture

Create `tests/test-fixtures/sample.py`:
```python
def hello_world():
    """A simple function."""
    pass

class Greeter:
    """A simple class."""
    pass

def another_function():
    pass
```

### Unit Test

```rust
#[test]
fn test_parse_python_file() {
    use crate::parser::{Parser, Language};
    use std::path::Path;

    let parser = Parser::new();
    let graph = parser.parse_file(
        Path::new("tests/test-fixtures/sample.py"),
        &Language::Python
    ).unwrap();

    // Should extract 2 functions + 1 class
    assert_eq!(graph.node_count(), 3);

    // Verify nodes exist (implementation can iterate graph to check)
}
```

## Acceptance Criteria

- [ ] `tree-sitter` and `tree-sitter-python` dependencies added
- [ ] `parse_file()` function compiles and runs
- [ ] Test fixture Python file created
- [ ] Unit test passes: extracts functions and classes correctly
- [ ] Can call `Parser::new().parse_file(path, Language::Python)` and get a Graph with nodes

## Next Epic (Theme Only)

**Epic 3: Graph Construction with Edges**

*Goal*: Add edge relationships (contains, calls, imports) to make the graph queriable.

*Rough scope*: Walk the tree-sitter tree to find relationships between nodes. Add edges for structural hierarchy (contains) and call relationships.

*Note*: Evaluate parser design after implementing second language—refactor to trait-based if enum becomes unwieldy.
