//! Import statement extraction from Python source files.
//!
//! This module provides data structures for capturing Python import statements
//! in a structured format, enabling cross-file dependency analysis without
//! re-parsing source code.
//!
//! # Architecture
//!
//! The import extraction follows a **lossless capture** philosophy - we preserve
//! all import syntax information so that downstream analysis (Epic 7) can make
//! accurate decisions without ambiguity.
//!
//! # Example
//!
//! ```
//! use std::path::Path;
//! use graph_migrator_core::import::{self, ImportStatement};
//!
//! # fn main() -> Result<(), anyhow::Error> {
//! let imports = import::extract_imports(Path::new("test/fixtures/imports/basic.py"))?;
//!
//! for import in &imports {
//!     match import {
//!         ImportStatement::Import { items, range } => {
//!             println!("Line {}: import {}", range.start_line,
//!                 items.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(", "));
//!         }
//!         ImportStatement::ImportFrom { module, level, names, range } => {
//!             let dots = ".".repeat(*level as usize);
//!             println!("Line {}: from {}{} import {}",
//!                 range.start_line, dots,
//!                 module.as_deref().unwrap_or(""),
//!                 names.len());
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parser::MultiFileGraph;

/// Map of file paths to their import statements.
///
/// Epic 7 uses this to resolve cross-file dependencies by correlating
/// import data with node provenance from `MultiFileGraph::node_locations`.
pub type ImportMap = HashMap<PathBuf, Vec<ImportStatement>>;

/// Combined output of Pass 1 (Epics 5 + 6).
///
/// This structure combines the symbol graph from Epic 5 with the import
/// data from Epic 6, providing Epic 7 with everything needed for
/// cross-file resolution.
///
/// Note: Does not derive `PartialEq`, `Serialize`, or `Deserialize` because
/// `MultiFileGraph` contains `StableGraph` which doesn't implement these traits.
/// For equality, compare `graph.node_count()` and `graph.edge_count()`.
#[derive(Debug, Clone)]
pub struct FirstPassOutput {
    /// The unified graph containing all nodes and edges from parsed files.
    pub graph: MultiFileGraph,

    /// Map of file paths to their import statements.
    ///
    /// Epic 7 uses this to resolve cross-file dependencies by correlating
    /// import data with node provenance from `graph.node_locations`.
    pub imports: ImportMap,
}

/// A single import statement from a Python file.
///
/// This enum provides lossless capture of Python import syntax,
/// enabling Epic 7 to perform accurate resolution without re-parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ImportStatement {
    /// `import module [as alias]`
    /// Also supports: `import a, b as c`
    Import {
        /// List of modules imported in this statement.
        /// Example: `import os, sys as system` → two ImportedModule items.
        items: Vec<ImportedModule>,
        /// Statement-level source location (MVP: per-item ranges deferred).
        range: SourceRange,
    },

    /// `from module import name [as alias]`
    /// Also supports: `from . import foo`, `from x import *`
    ImportFrom {
        /// Module name (None for `from . import foo`).
        /// `None` means the import is from the current package (relative import with level > 0).
        module: Option<String>,
        /// Relative import level (0 = absolute, 1 = `.`, 2 = `..`).
        /// Stored but not resolved in Epic 6.
        level: u8,
        /// Imported symbols (may include star import).
        names: Vec<ImportedName>,
        /// Statement-level source location (MVP: per-item ranges deferred).
        range: SourceRange,
    },
}

/// A single module imported via `import` statement.
///
/// Represents one item in `import x, y, z` syntax.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedModule {
    /// Module name (e.g., "os", "sys", "numpy").
    pub name: String,
    /// Alias if present (e.g., `import numpy as np` → Some("np")).
    pub alias: Option<String>,
}

/// A single symbol imported via `from ... import` statement.
///
/// Represents one item in `from x import a, b, c` syntax.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedName {
    /// Symbol name (e.g., "path", "join", or "*" for star imports).
    pub name: String,
    /// Alias if present (e.g., `from os import path as p` → Some("p")).
    pub alias: Option<String>,
    /// Whether this is a star import (`from module import *`).
    ///
    /// Star imports require special handling in Epic 7 since we cannot
    /// determine at parse time which symbols are actually imported.
    #[serde(default)]
    pub is_star: bool,
}

/// Source location in a file.
///
/// Provides statement-level source ranges for error reporting and
/// IDE integration purposes.
///
/// MVP: Statement-level ranges only. Per-item ranges can be added later
/// if "go to definition" features are needed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceRange {
    /// Byte offset of start (0-based).
    pub start_byte: usize,
    /// Byte offset of end (0-based).
    pub end_byte: usize,
    /// Start line (1-indexed for human readability).
    pub start_line: usize,
    /// End line (1-indexed for human readability).
    pub end_line: usize,
}

/// Extract import statements from a Python file.
///
/// Parses the file using tree-sitter and walks the AST to extract
/// all import statements into structured data.
///
/// # Arguments
///
/// * `path` - Path to the Python file to analyze
///
/// # Returns
///
/// A vector of `ImportStatement` structs representing all imports in the file,
/// ordered by their appearance in the source file.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The file cannot be parsed by tree-sitter Python parser
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use graph_migrator_core::import;
///
/// # fn main() -> Result<(), anyhow::Error> {
/// let imports = import::extract_imports(Path::new("test/fixtures/imports/complex.py"))?;
/// println!("Found {} import statements", imports.len());
/// # Ok(())
/// # }
/// ```
pub fn extract_imports(_path: &Path) -> anyhow::Result<Vec<ImportStatement>> {
    todo!("Tree-sitter parsing implementation pending")
}

/// Parse all Python files in a directory and extract both graph and imports.
///
/// This convenience function combines Epic 5's `parse_directory()` with
/// Epic 6's `extract_imports()` to produce a unified `FirstPassOutput`.
///
/// # Arguments
///
/// * `root` - Root directory to search and parse
///
/// # Returns
///
/// A `FirstPassOutput` containing the merged graph and import map.
///
/// # Example
///
/// ```
/// use std::path::Path;
/// use graph_migrator_core::import;
///
/// # fn main() -> Result<(), anyhow::Error> {
/// let output = import::parse_directory_with_imports(Path::new("my_project"))?;
///
/// println!("Parsed {} nodes from {} files",
///     output.graph.graph.node_count(),
///     output.graph.file_nodes.len()
/// );
///
/// println!("Found {} files with imports", output.imports.len());
/// # Ok(())
/// # }
/// ```
pub fn parse_directory_with_imports(root: &Path) -> anyhow::Result<FirstPassOutput> {
    use crate::parser;

    let graph = parser::parse_directory(root)?;

    let mut imports = ImportMap::new();
    for file_path in &graph.file_nodes {
        let file_imports = extract_imports(file_path)?;
        imports.insert(file_path.clone(), file_imports);
    }

    Ok(FirstPassOutput { graph, imports })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[allow(dead_code)]
    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_import_statement_structure() {
        let stmt = ImportStatement::Import {
            items: vec![
                ImportedModule {
                    name: "os".to_string(),
                    alias: None,
                },
                ImportedModule {
                    name: "sys".to_string(),
                    alias: Some("system".to_string()),
                },
            ],
            range: SourceRange {
                start_byte: 0,
                end_byte: 25,
                start_line: 1,
                end_line: 1,
            },
        };

        match stmt {
            ImportStatement::Import { items, range } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].name, "os");
                assert_eq!(items[1].alias, Some("system".to_string()));
                assert_eq!(range.start_line, 1);
            }
            _ => panic!("Expected Import variant"),
        }
    }

    #[test]
    fn test_import_from_statement_structure() {
        let stmt = ImportStatement::ImportFrom {
            module: Some("os".to_string()),
            level: 0,
            names: vec![ImportedName {
                name: "path".to_string(),
                alias: None,
                is_star: false,
            }],
            range: SourceRange {
                start_byte: 0,
                end_byte: 20,
                start_line: 1,
                end_line: 1,
            },
        };

        match stmt {
            ImportStatement::ImportFrom {
                module,
                level,
                names,
                ..
            } => {
                assert_eq!(module, Some("os".to_string()));
                assert_eq!(level, 0);
                assert_eq!(names.len(), 1);
                assert_eq!(names[0].name, "path");
            }
            _ => panic!("Expected ImportFrom variant"),
        }
    }

    #[test]
    fn test_relative_import_structure() {
        let stmt = ImportStatement::ImportFrom {
            module: None,
            level: 1,
            names: vec![ImportedName {
                name: "helper".to_string(),
                alias: None,
                is_star: false,
            }],
            range: SourceRange {
                start_byte: 0,
                end_byte: 23,
                start_line: 1,
                end_line: 1,
            },
        };

        match stmt {
            ImportStatement::ImportFrom {
                module,
                level,
                names,
                ..
            } => {
                assert_eq!(module, None);
                assert_eq!(level, 1);
                assert_eq!(names[0].name, "helper");
            }
            _ => panic!("Expected ImportFrom variant"),
        }
    }

    #[test]
    fn test_star_import_structure() {
        let stmt = ImportStatement::ImportFrom {
            module: Some("typing".to_string()),
            level: 0,
            names: vec![ImportedName {
                name: "*".to_string(),
                alias: None,
                is_star: true,
            }],
            range: SourceRange {
                start_byte: 0,
                end_byte: 26,
                start_line: 1,
                end_line: 1,
            },
        };

        match stmt {
            ImportStatement::ImportFrom { names, .. } => {
                assert!(names[0].is_star);
                assert_eq!(names[0].name, "*");
            }
            _ => panic!("Expected ImportFrom variant"),
        }
    }

    #[test]
    fn test_source_range_structure() {
        let range = SourceRange {
            start_byte: 100,
            end_byte: 250,
            start_line: 5,
            end_line: 7,
        };

        assert_eq!(range.start_byte, 100);
        assert_eq!(range.end_byte, 250);
        assert_eq!(range.start_line, 5);
        assert_eq!(range.end_line, 7);
    }

    #[test]
    fn test_first_pass_output_structure() {
        let output = FirstPassOutput {
            graph: MultiFileGraph::default(),
            imports: ImportMap::new(),
        };

        assert_eq!(output.graph.graph.node_count(), 0);
        assert!(output.imports.is_empty());
    }

    #[test]
    fn test_import_map_insertion() {
        let mut map = ImportMap::new();
        let path = PathBuf::from("/test/file.py");

        let imports = vec![ImportStatement::Import {
            items: vec![ImportedModule {
                name: "os".to_string(),
                alias: None,
            }],
            range: SourceRange {
                start_byte: 0,
                end_byte: 6,
                start_line: 1,
                end_line: 1,
            },
        }];

        map.insert(path.clone(), imports.clone());

        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&path).unwrap().len(), 1);
    }

    #[test]
    fn test_import_statement_serialization() {
        let stmt = ImportStatement::Import {
            items: vec![ImportedModule {
                name: "os".to_string(),
                alias: None,
            }],
            range: SourceRange {
                start_byte: 0,
                end_byte: 6,
                start_line: 1,
                end_line: 1,
            },
        };

        let serialized = serde_json::to_string(&stmt).unwrap();
        let deserialized: ImportStatement = serde_json::from_str(&serialized).unwrap();

        assert_eq!(stmt, deserialized);
    }

    #[test]
    fn test_import_from_statement_serialization() {
        let stmt = ImportStatement::ImportFrom {
            module: Some("os".to_string()),
            level: 0,
            names: vec![ImportedName {
                name: "path".to_string(),
                alias: Some("p".to_string()),
                is_star: false,
            }],
            range: SourceRange {
                start_byte: 0,
                end_byte: 25,
                start_line: 1,
                end_line: 1,
            },
        };

        let serialized = serde_json::to_string(&stmt).unwrap();
        let deserialized: ImportStatement = serde_json::from_str(&serialized).unwrap();

        assert_eq!(stmt, deserialized);
    }
}
