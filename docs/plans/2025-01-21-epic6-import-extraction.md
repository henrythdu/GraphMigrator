# Epic 6: Import Statement Extraction

**Date**: 2025-01-21
**Status**: Ready to Implement
**Phase**: Phase 1 (MVP)

## Goal

Extract import statements from Python AST into structured data, using patterns borrowed from Ruff's import model.

> **Positioning Statement**: This epic focuses on extracting and structuring import data. It does NOT resolve imports to files or create graph edges—that's Epic 7.

## Scope

### What Epic 6 Does

- Parse `import_statement` nodes from tree-sitter AST
- Parse `import_from_statement` nodes from tree-sitter AST
- Create Ruff-inspired `ImportStatement` data structures (lossless capture)
- Handle aliases (`import numpy as np`)
- Support multiple imports per statement (`import os, sys`)
- Support star imports (`from module import *`)
- Support relative imports (`from . import helper`)
- Track statement-level source locations
- Return structured import data per file
- **NEW**: Provide `FirstPassOutput` struct combining Epic 5's graph with Epic 6's imports

### What Epic 6 Does NOT Do (YAGNI)

- **No import resolution** - that's Epic 7
- **No file path resolution** - that's Epic 7
- **No creating EdgeType::Imports edges** - that's Epic 7
- **No cross-file symbol resolution** - that's Epic 7
- **No per-item ranges** (MVP simplification - statement-level ranges only)

## Architecture (High-Level)

**Borrow from Ruff**:
- Enum pattern: `ImportStatement::Import()` vs `ImportStatement::ImportFrom()`
- Support for multiple imports per statement
- `level` field for relative imports (store but don't resolve yet)
- Statement-level source ranges (MVP - per-item ranges deferred)

**Integration with Epic 5**:

Based on PAL challenge + thinkdeep analysis, Epic 6 uses a **parallel structure approach** to integrate with Epic 5's `MultiFileGraph`:

```rust
/// Map of file paths to their import statements
pub type ImportMap = HashMap<PathBuf, Vec<ImportStatement>>;

/// Combined output of Pass 1 (Epics 5 + 6)
///
/// This structure combines the symbol graph from Epic 5 with the import
/// data from Epic 6, providing Epic 7 with everything needed for
/// cross-file resolution.
pub struct FirstPassOutput {
    /// The unified graph containing all nodes and edges from parsed files
    pub graph: MultiFileGraph,

    /// Map of file paths to their import statements
    ///
    /// Epic 7 uses this to resolve cross-file dependencies by correlating
    /// import data with node provenance from `graph.node_locations`.
    pub imports: ImportMap,
}
```

**Data structures** (simplified based on PAL challenge analysis):
```rust
/// A single import statement from a Python file
///
/// This enum provides lossless capture of Python import syntax,
/// enabling Epic 7 to perform accurate resolution without re-parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportStatement {
    /// `import module [as alias]`
    /// Also supports: `import a, b as c`
    Import {
        items: Vec<ImportedModule>,
        /// Statement-level source location (MVP: per-item ranges deferred)
        range: SourceRange,
    },

    /// `from module import name [as alias]`
    /// Also supports: `from . import foo`, `from x import *`
    ImportFrom {
        /// Module name (None for `from . import foo`)
        module: Option<String>,
        /// Relative import level (0 = absolute, 1 = `.`, 2 = `..`)
        level: u8,
        /// Imported symbols (may include star import)
        names: Vec<ImportedName>,
        /// Statement-level source location (MVP: per-item ranges deferred)
        range: SourceRange,
    },
}

/// A single module imported via `import` statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedModule {
    /// Module name (e.g., "os", "sys")
    pub name: String,
    /// Alias if present (e.g., `import numpy as np` → Some("np"))
    pub alias: Option<String>,
}

/// A single symbol imported via `from ... import` statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedName {
    /// Symbol name (e.g., "path", "join", or "*" for star imports)
    pub name: String,
    /// Alias if present (e.g., `from os import path as p` → Some("p"))
    pub alias: Option<String>,
    /// Whether this is a star import (`from module import *`)
    pub is_star: bool,
}

/// Source location in a file
///
/// MVP: Statement-level ranges only. Per-item ranges can be added later
/// if "go to definition" features are needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceRange {
    /// Byte offset of start
    pub start_byte: usize,
    /// Byte offset of end
    pub end_byte: usize,
    /// Start line (1-indexed)
    pub start_line: usize,
    /// End line (1-indexed)
    pub end_line: usize,
}
```

**API**:
```rust
// crates/core/src/import.rs

/// Extract import statements from a Python file
///
/// # Arguments
/// * `path` - Path to the Python file to analyze
///
/// # Returns
/// A vector of `ImportStatement` structs representing all imports in the file
///
/// # Example
/// ```no_run
/// use graph_migrator_core::import;
///
/// let imports = import::extract_imports(Path::new("main.py"))?;
///
/// for import in imports {
///     match import {
///         ImportStatement::Import { items, range } => {
///             println!("Line {}: {} imports", range.start_line, items.len());
///         }
///         ImportStatement::ImportFrom { module, level, names, range } => {
///             let dots = ".".repeat(level as usize);
///             println!("Line {}: from {}{} import {} items",
///                 range.start_line,
///                 dots,
///                 module.as_deref().unwrap_or(""),
///                 names.len()
///             );
///         }
///     }
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn extract_imports(path: &Path) -> anyhow::Result<Vec<ImportStatement>> {
    // Parse file with tree-sitter
    // Walk AST for import_statement and import_from_statement
    // Map to ImportStatement structs with statement-level ranges
}

/// Parse all Python files in a directory and extract both graph and imports
///
/// This convenience function combines Epic 5's `parse_directory()` with
/// Epic 6's `extract_imports()` to produce a unified `FirstPassOutput`.
///
/// # Arguments
/// * `root` - Root directory to search and parse
///
/// # Returns
/// A `FirstPassOutput` containing the merged graph and import map
///
/// # Example
/// ```no_run
/// use graph_migrator_core::import;
///
/// let output = import::parse_directory_with_imports(Path::new("my_project"))?;
///
/// println!("Parsed {} nodes from {} files with {} imports",
///     output.graph.graph.node_count(),
///     output.graph.file_nodes.len(),
///     output.imports.len()
/// );
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_directory_with_imports(root: &Path) -> anyhow::Result<FirstPassOutput> {
    use crate::parser;

    // Parse all files to get the graph
    let graph = parser::parse_directory(root)?;

    // Extract imports for each file
    let mut imports = ImportMap::new();
    for file_path in &graph.file_nodes {
        let file_imports = extract_imports(file_path)?;
        imports.insert(file_path.clone(), file_imports);
    }

    Ok(FirstPassOutput { graph, imports })
}
```

## Implementation Tasks

**Task 1: Add import module and data structures**
- [ ] Create `crates/core/src/import.rs` module
- [ ] Add `ImportStatement` enum with `Import` and `ImportFrom` variants
- [ ] Add `ImportedModule`, `ImportedName`, and `SourceRange` structs
- [ ] Add `ImportMap` type alias
- [ ] Add `FirstPassOutput` struct
- [ ] Implement `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` derives
- [ ] Add comprehensive doc comments

**Task 2: Implement import extraction logic**
- [ ] Extend `python.rs` with `extract_imports()` function
- [ ] Use tree-sitter to parse Python files
- [ ] Walk AST for `import_statement` and `import_from_statement` nodes
- [ ] Map tree-sitter nodes to `ImportStatement` structs
- [ ] Extract statement-level source ranges
- [ ] Handle all Python import syntax variants (see test fixtures)

**Task 3: Handle Python import edge cases**
- [ ] Multiple imports per statement: `import os, sys as system`
- [ ] Star imports: `from module import *`
- [ ] Relative imports: `from . import helper`, `from ..pkg import foo`
- [ ] Parenthesized multi-line imports: `from x import (a, b)`
- [ ] Mixed aliases: `from x import y as z, a`

**Task 4: Implement FirstPassOutput integration**
- [ ] Implement `parse_directory_with_imports()` function
- [ ] Combine Epic 5's `parse_directory()` with import extraction
- [ ] Build `ImportMap` from all files in `graph.file_nodes`
- [ ] Return `FirstPassOutput` with both graph and imports
- [ ] Add unit tests for integration

**Task 5: Write comprehensive unit tests**
- [ ] Test basic imports: `import os`
- [ ] Test aliased imports: `import numpy as np`
- [ ] Test from imports: `from os import path`
- [ ] Test multiple imports: `import os, sys`
- [ ] Test star imports: `from module import *`
- [ ] Test relative imports: `from . import helper`
- [ ] Test multi-line parenthesized imports
- [ ] Test edge cases: `from . import *`, `from .. import foo`

**Task 6: Integration test**
- [ ] Create multi-file test fixture with various import patterns
- [ ] Verify all import types are captured correctly
- [ ] Verify source ranges are accurate
- [ ] Test `FirstPassOutput` integration
- [ ] Test lossless capture (no information loss)

**Task 7: Documentation**
- [ ] Add module-level documentation explaining the parallel structure approach
- [ ] Document star import handling as Epic 7 requirement
- [ ] Document relative import `level` field
- [ ] Document `FirstPassOutput` purpose for Epic 7
- [ ] Add examples in doc comments
- [ ] Document MVP simplification (statement-level ranges only)

## Python Import Syntax Coverage

Epic 6 must handle the following Python import syntaxes:

| Syntax | Example | Data Structure |
|--------|---------|----------------|
| Basic import | `import os` | `Import { items: [ImportedModule { name: "os", alias: None }], range: ... }` |
| Aliased import | `import numpy as np` | `Import { items: [ImportedModule { name: "numpy", alias: Some("np") }], range: ... }` |
| Multiple imports | `import os, sys` | `Import { items: [ImportedModule { name: "os", ... }, ImportedModule { name: "sys", ... }], range: ... }` |
| Mixed aliases | `import a, b as c` | `Import { items: [ImportedModule { name: "a", ... }, ImportedModule { name: "b", alias: Some("c"), ... }], range: ... }` |
| From import | `from os import path` | `ImportFrom { module: Some("os"), level: 0, names: [ImportedName { name: "path", ... }], range: ... }` |
| From with alias | `from os import path as p` | `ImportFrom { module: Some("os"), level: 0, names: [ImportedName { name: "path", alias: Some("p"), ... }], range: ... }` |
| From multiple | `from os import path, join` | `ImportFrom { module: Some("os"), level: 0, names: [ImportedName { name: "path", ... }, ImportedName { name: "join", ... }], range: ... }` |
| Star import | `from module import *` | `ImportFrom { module: Some("module"), level: 0, names: [ImportedName { name: "*", is_star: true, ... }], range: ... }` |
| Relative import | `from . import helper` | `ImportFrom { module: None, level: 1, names: [ImportedName { name: "helper", ... }], range: ... }` |
| Relative from | `from ..pkg import foo` | `ImportFrom { module: Some("pkg"), level: 2, names: [ImportedName { name: "foo", ... }], range: ... }` |
| Multi-line | `from x import (a, b)` | Same as `from x import a, b` |
| Multi-line star | `from x import (*)` | Same as `from x import *` |

## Acceptance Criteria

Epic 6 is complete when:

- [ ] `extract_imports()` returns structured import data from a file
- [ ] All Python import syntax variants in the table above are supported
- [ ] Multiple imports per statement are captured as `Vec<ImportedModule>`
- [ ] Star imports include `is_star: true` flag
- [ ] Relative imports have `module: None` and correct `level` value
- [ ] Each import statement has accurate `SourceRange` location
- [ ] `FirstPassOutput` combines `MultiFileGraph` with `ImportMap`
- [ ] `parse_directory_with_imports()` returns valid `FirstPassOutput`
- [ ] Unit tests cover all import syntax variants
- [ ] Integration test with multi-file fixture passes
- [ ] All Epic 3-5 tests still pass (backward compatibility)

## What Success Looks Like

After Epic 6:

```rust
use graph_migrator_core::import;

// Parse a project and get both graph and imports
let output = parse_directory_with_imports(Path::new("my_project"))?;

// Access the graph (Epic 5 output)
println!("Parsed {} nodes from {} files",
    output.graph.graph.node_count(),
    output.graph.file_nodes.len()
);

// Access the imports (Epic 6 output)
println!("Found {} files with imports", output.imports.len());

// Query: What does main.py import?
if let Some(imports) = output.imports.get(Path::new("my_project/main.py")) {
    for import in imports {
        match import {
            ImportStatement::Import { items, range } => {
                for item in items {
                    println!("  import {}{}",
                        item.name,
                        item.alias.as_ref().map(|a| format!(" as {}", a)).unwrap_or_default()
                    );
                }
            }
            ImportStatement::ImportFrom { module, level, names, .. } => {
                let dots = ".".repeat(level as usize);
                for name in names {
                    println!("  from {}{}{} import {}",
                        dots,
                        module.as_deref().unwrap_or(""),
                        if module.is_some() { "" } else { " " },
                        name.name
                    );
                }
            }
        }
    }
}

// Epic 7 will use output.graph.node_locations + output.imports
// to resolve cross-file dependencies
```

**Epic 7 will then use `FirstPassOutput` to:**
- Resolve `module` to actual file paths using `imports` map
- Cross-reference with `graph.node_locations` for provenance
- Create `EdgeType::Imports` edges between modules
- Handle star imports as special case
- Resolve cross-file function calls

## PAL Consensus Results

**Date**: 2026-01-21
**Models Consulted**: Gemini 2.5 Pro (for), GPT-5.2 (against), Claude Opus 4.5 (neutral)
**Overall Confidence**: **High (8-9/10)**

### Key Findings

**✅ All models agreed on:**
1. YAGNI boundaries are correct - Epic 6 extracts, Epic 7 resolves
2. PRD alignment confirmed - enables multi-file dependency tracking
3. Architectural purity - clean separation between Epics 5, 6, and 7
4. Ruff-inspired approach is appropriate and reduces risk

**🔧 Critical refinements from GPT-5.2 (against stance):**
1. Support multiple imports per statement (`Vec<ImportedModule>`)
2. Include `module: Option<String>` for relative imports
3. Add `is_star` flag for star imports
4. ~~Add per-item source ranges~~ → **Simplified to statement-level ranges (MVP)**

**🔗 Epic 5 integration concern (RESOLVED):**
GPT-5.2 recommended clarifying how import data ties into Epic 5's `MultiFileGraph` / provenance model.

**Solution (from PAL challenge + thinkdeep analysis)**:
- Use **parallel structure approach** with `FirstPassOutput`
- Keep `MultiFileGraph` unchanged (lower risk, clean separation)
- `ImportMap` provides lookup table keyed by file path
- Epic 7 correlates both using file path as the key

### Verdict

**READY TO IMPLEMENT** with refined data structures for lossless capture and parallel structure integration.

The refinements ensure Epic 7 can perform accurate resolution without re-parsing or dealing with lossy data structures.

## PAL Challenge + ThinkDeep Analysis

**Date**: 2026-01-21
**Analysis Type**: Critical design review

### Challenge Questions Investigated

**Q1: Is Epic 6 actually needed?**
✅ **YES - Essential**. Epic 7 cannot resolve cross-file calls without import data. On-the-fly extraction would require re-parsing (inefficient) or violate SRP.

**Q2: Are we over-engineering?**
✅ **Mostly appropriate**. One simplification made:
- Per-item ranges → **Simplified to statement-level ranges (MVP)**

**Q3: Epic 5 integration?**
✅ **RESOLVED** with `FirstPassOutput` parallel structure approach:
- Clean separation: symbols → graph, imports → lookup table
- Lower risk: no modification to stable `MultiFileGraph`
- Easy to extend: can add more per-file metadata later

**Q4: PRD alignment?**
✅ **ALIGNED** - Epic 6 directly enables "which functions across 4 files" goal.

**Q5: Alternative approaches?**
| Option | Description | Verdict |
|--------|-------------|---------|
| A | Store imports in MultiFileGraph | ⚠️ Premature optimization |
| **B** | **Parallel structure (FirstPassOutput)** | ✅ **CHOSEN** |
| C | Do extraction in Epic 7 | ❌ Violates SRP |

### Key Recommendations Implemented

1. **Simplified ranges**: Statement-level only (MVP - per-item ranges can be added later)
2. **Added `FirstPassOutput`**: Combines Epic 5's graph with Epic 6's imports
3. **Added `ImportMap`**: Type alias for `HashMap<PathBuf, Vec<ImportStatement>>`
4. **Added `parse_directory_with_imports()`**: Convenience function for complete Pass 1 output

## Dependencies

**On Epic 5**:
- Uses tree-sitter parsing infrastructure from Epic 3
- Builds on `MultiFileGraph` and provenance tracking concept
- `FirstPassOutput` wraps `MultiFileGraph` without modifying it

**Enables Epic 7**:
- Provides `FirstPassOutput` with both graph and imports
- Import data enables resolution without re-parsing
- Star import flag enables special handling
- Statement-level ranges support error reporting

## Next Epic (Theme Only)

**Epic 7: Import Resolution and Cross-File Edges**

*Goal*: Resolve imports to actual file paths and create `EdgeType::Imports` edges in the graph. Enable cross-file call resolution using a symbol index.

*Input*: `FirstPassOutput` from Epic 6 (contains both `MultiFileGraph` and `ImportMap`).

*Theme*: Import resolution → file paths → graph edges + cross-file call edges. This completes the multi-file dependency graph.
