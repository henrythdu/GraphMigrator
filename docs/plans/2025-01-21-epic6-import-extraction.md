# Epic 6: Import Statement Extraction

**Date**: 2025-01-21
**Status**: Theme Only
**Phase**: Phase 1 (MVP)

## Goal

Extract import statements from Python AST into structured data, using patterns borrowed from Ruff's import model.

> **Positioning Statement**: This epic focuses on extracting and structuring import data. It does NOT resolve imports to files or create graph edges—that's Epic 7.

## Scope

### What Epic 6 Does

- Parse `import_statement` nodes from tree-sitter AST
- Parse `import_from_statement` nodes from tree-sitter AST
- Create Ruff-inspired `ImportStatement` data structures
- Handle aliases (`import numpy as np`)
- Return structured import data per file

### What Epic 6 Does NOT Do (YAGNI)

- **No import resolution** - that's Epic 7
- **No file path resolution** - that's Epic 7
- **No creating EdgeType::Imports edges** - that's Epic 7
- **No cross-file symbol resolution** - that's Epic 7

## Architecture (High-Level)

**Borrow from Ruff**:
- Enum pattern: `ImportStatement::Import()` vs `ImportStatement::ImportFrom()`
- `Alias` struct for tracking renamed imports
- `level` field for relative imports (store but don't resolve yet)

**Data structures**:
```rust
pub enum ImportStatement {
    Import(ModuleImport),
    ImportFrom(FromImport),
}

pub struct ModuleImport {
    pub module: String,
    pub alias: Option<String>,
}

pub struct FromImport {
    pub module: Option<String>,
    pub symbols: Vec<ImportedSymbol>,
    pub level: u32,  // Store but don't use yet
}
```

**API**:
```rust
pub fn extract_imports(path: &Path) -> anyhow::Result<Vec<ImportStatement>> {
    // Parse file with tree-sitter
    // Walk AST for import_statement and import_from_statement
    // Map to ImportStatement structs
}
```

## Implementation Notes

- Add `crates/core/src/import.rs` module for data structures
- Extend `python.rs` with import extraction logic
- Use tree-sitter queries or manual tree walking
- Store relative import `level` but don't resolve (document as limitation)

## Acceptance Criteria

Epic 6 is complete when:

- [ ] `extract_imports()` returns structured import data from a file
- [ ] `import os` → `ImportStatement::Import(ModuleImport { module: "os", alias: None })`
- [ ] `import numpy as np` → captures `alias: Some("np")`
- [ ] `from os import path` → `ImportStatement::ImportFrom(...)`
- [ ] Unit tests cover various import syntaxes
- [ ] All Epic 3-5 tests still pass

## What Success Looks Like

After Epic 6:

```rust
let imports = extract_imports(Path::new("main.py"))?;

for import in imports {
    println!("{:?}", import);
    // ImportStatement::Import(ModuleImport { module: "os", ... })
    // ImportStatement::ImportFrom(FromImport { module: Some("os"), ... })
}
```

## Next Epic (Theme Only)

**Epic 7: Import Resolution and Cross-File Edges**

*Goal*: Resolve imports to actual file paths and create `EdgeType::Imports` edges in the graph. Enable cross-file call resolution using a symbol index.

*Theme*: Import resolution → file paths → graph edges + cross-file call edges. This completes the multi-file dependency graph.
