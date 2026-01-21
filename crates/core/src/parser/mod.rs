//! Parser module for extracting code structure from source files
//!
//! This module provides language-specific parsers using tree-sitter
//! to build dependency graphs from source code.
//!
//! # Two-Pass Architecture
//!
//! This module implements **Pass 1** of a two-pass architecture for multi-file parsing:
//! - **Pass 1 (Epic 5)**: Parse all files, collect symbols, establish provenance
//! - **Pass 2 (Epic 7)**: Resolve imports, create cross-file edges
//!
//! The key insight: "not doing import resolution" doesn't mean "not collecting
//! the metadata needed for resolution." Provenance tracking (node → file mapping)
//! is essential foundation, not optional complexity.
//!
//! # NodeId Format
//!
//! Node IDs use the format `file_path::symbol_name` (e.g., `src/utils.py::helper`).
//! This format ensures **global uniqueness** across all files:
//!
//! - `file1.py::helper` and `file2.py::helper` have **different** IDs → both kept
//! - Same file parsed twice → same ID → first wins (correct deduplication)
//!
//! This property is critical for the graph merging strategy: deduplication by ID
//! works correctly because IDs incorporate the file path.
//!
//! # Multi-File Parsing API
//!
//! - [`parse_files()`] - Parse multiple specific files into a unified graph
//! - [`parse_directory()`] - Discover and parse all Python files in a directory
//! - [`MultiFileGraph`] - Result structure with graph + provenance metadata

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub mod python;

/// Supported programming languages for parsing
pub enum Language {
    Python,
}

/// Parser for building dependency graphs from source code
pub struct Parser;

impl Parser {
    /// Create a new parser instance
    pub fn new() -> Self {
        Parser
    }

    /// Parse a source file and extract its structure into a graph
    ///
    /// # Arguments
    /// * `path` - Path to the source file to parse
    /// * `lang` - The programming language of the source file
    ///
    /// # Returns
    /// A `Graph` containing nodes for extracted symbols
    pub fn parse_file(&self, path: &Path, lang: &Language) -> anyhow::Result<crate::Graph> {
        match lang {
            Language::Python => python::parse_file(path),
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of parsing multiple files into a unified graph
///
/// This structure is designed as the foundation for Epic 7's two-pass
/// resolution. It contains the merged graph plus provenance metadata
/// needed for future cross-file edge creation.
pub struct MultiFileGraph {
    /// The unified graph containing all nodes and edges from parsed files
    pub graph: crate::Graph,

    /// Maps node IDs to their NodeIndex for O(1) lookups
    ///
    /// This internal cache avoids the O(N) linear scan that `find_node_by_id()` performs.
    /// Essential for performance when merging large graphs.
    node_id_map: HashMap<String, petgraph::stable_graph::NodeIndex>,

    /// Maps each node ID to its source file path
    ///
    /// **Key format**: Node IDs use `file_path::symbol_name` format (e.g., `src/utils.py::helper`).
    /// This ensures global uniqueness across files.
    ///
    /// This is NOT import resolution - it's provenance metadata that
    /// Epic 7 will use when resolving cross-file dependencies.
    ///
    /// # Purpose
    /// Answer "where is this node defined?" for impact analysis.
    ///
    /// # Example
    /// ```text
    /// node_locations = {
    ///     "src/utils.py::helper" → "/project/src/utils.py",
    ///     "src/main.py::main" → "/project/src/main.py",
    /// }
    /// ```
    pub node_locations: HashMap<String, PathBuf>,

    /// Reverse lookup: tracks which files have been parsed
    ///
    /// Purpose: Enable efficient "what files have been parsed?" queries.
    /// In future epics, this can be extended to map to file-node indices
    /// when File nodes are added to the parser.
    pub file_nodes: HashSet<PathBuf>,
}

impl MultiFileGraph {
    /// Create a new empty MultiFileGraph
    pub fn new() -> Self {
        Self {
            graph: crate::Graph::new(),
            node_id_map: HashMap::new(),
            node_locations: HashMap::new(),
            file_nodes: HashSet::new(),
        }
    }

    /// Merge a single-file graph into this multi-file graph
    ///
    /// Handles node deduplication and edge index remapping.
    ///
    /// # Arguments
    /// * `file_graph` - The graph from a single file to merge
    /// * `source_file` - The path to the source file (for provenance tracking)
    ///
    /// # Behavior
    /// - Node deduplication: If a node with the same ID already exists, the existing
    ///   node is used (first occurrence wins). This is safe because NodeId format is
    ///   `file_path::symbol_name`, making globally unique.
    /// - Edge remapping: Edge endpoints are remapped to use the correct node indices
    ///   in the merged graph.
    /// - Provenance tracking: `node_locations` maps each node ID to its source file.
    pub fn merge_file_graph(&mut self, file_graph: crate::Graph, source_file: &Path) -> anyhow::Result<()> {
        use petgraph::stable_graph::NodeIndex;

        let mut index_map: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        // Track all files that have been merged
        self.file_nodes.insert(source_file.to_path_buf());

        // Add nodes (skip duplicates, track index mappings)
        for node_idx in file_graph.node_indices() {
            let node = file_graph.node_weight(node_idx)
                .ok_or_else(|| anyhow::anyhow!("Invalid node index in file graph"))?;

            if let Some(&existing_idx) = self.node_id_map.get(&node.id) {
                // Duplicate: use existing node
                index_map.insert(node_idx, existing_idx);
            } else {
                // New node: add to graph
                let new_idx = self.graph.add_node(node.clone());
                index_map.insert(node_idx, new_idx);

                // Track in our ID map for O(1) lookups
                self.node_id_map.insert(node.id.clone(), new_idx);

                // Track provenance
                self.node_locations.insert(node.id.clone(), source_file.to_path_buf());
            }
        }

        // Add edges (remap indices)
        for edge_idx in file_graph.edge_indices() {
            let (source, target) = file_graph.edge_endpoints_for(edge_idx)
                .ok_or_else(|| anyhow::anyhow!("Invalid edge index in file graph"))?;

            let new_source = index_map.get(&source)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("Source node index not in mapping"))?;
            let new_target = index_map.get(&target)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("Target node index not in mapping"))?;

            let edge_weight = file_graph.edge_weight(edge_idx)
                .ok_or_else(|| anyhow::anyhow!("Invalid edge weight"))?;

            self.graph.add_edge(new_source, new_target, edge_weight.clone());
        }

        Ok(())
    }
}

impl Default for MultiFileGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse multiple Python files into a unified multi-file graph
///
/// # Arguments
/// * `paths` - Slice of file paths to parse
///
/// # Returns
/// A `MultiFileGraph` containing the merged graph and provenance metadata
///
/// # Behavior
/// - Paths are sorted for deterministic merging (same input → same output)
/// - Each file is parsed using the Python parser
/// - Individual file graphs are merged into the unified graph
/// - Provenance metadata tracks which file each node came from
///
/// # Example
/// ```no_run
/// use graph_migrator_core::parser;
///
/// let files = vec![
///     std::path::Path::new("src/main.py"),
///     std::path::Path::new("src/utils.py"),
/// ];
///
/// let multi = parser::parse_files(&files).unwrap();
/// println!("Parsed {} nodes from {} files",
///          multi.graph.node_count(),
///          multi.file_nodes.len());
/// ```
pub fn parse_files(paths: &[&Path]) -> anyhow::Result<MultiFileGraph> {
    let mut multi_graph = MultiFileGraph::new();

    // Sort paths for deterministic merging
    let mut sorted_paths: Vec<&Path> = paths.to_vec();
    sorted_paths.sort();

    // Create parser once outside the loop
    let parser = Parser::new();
    for path in sorted_paths {
        let file_graph = parser.parse_file(path, &Language::Python)?;
        multi_graph.merge_file_graph(file_graph, path)?;
    }

    Ok(multi_graph)
}

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
/// let multi = parser::parse_directory(std::path::Path::new("my_project")).unwrap();
/// println!("Parsed {} nodes", multi.graph.node_count());
/// ```
pub fn parse_directory(root: &Path) -> anyhow::Result<MultiFileGraph> {
    use crate::discovery;

    let files = discovery::discover_python_files(root);

    // Convert Vec<PathBuf> to Vec<&Path>
    let file_refs: Vec<&Path> = files.iter().map(|p| p.as_path()).collect();

    parse_files(&file_refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_files_single_file() {
        let files = vec![Path::new("tests/test-fixtures/sample.py")];
        let multi = parse_files(&files).unwrap();

        assert_eq!(multi.file_nodes.len(), 1);
        assert!(multi.graph.node_count() > 0);

        // Verify node_locations has entries
        assert!(!multi.node_locations.is_empty());
    }

    #[test]
    fn test_parse_files_multiple_files() {
        let files = vec![
            Path::new("tests/test-fixtures/sample.py"),
            Path::new("tests/test-fixtures/nested.py"),
        ];
        let multi = parse_files(&files).unwrap();

        assert_eq!(multi.file_nodes.len(), 2);

        // Verify nodes from both files are present
        // sample.py has 3 nodes, nested.py has 2 nodes
        assert!(multi.graph.node_count() >= 5);
    }

    #[test]
    fn test_multi_file_project() {
        let files = vec![
            Path::new("tests/test-fixtures/multi-file-project/module_a.py"),
            Path::new("tests/test-fixtures/multi-file-project/module_b.py"),
            Path::new("tests/test-fixtures/multi-file-project/main.py"),
        ];
        let multi = parse_files(&files).unwrap();

        assert_eq!(multi.file_nodes.len(), 3);

        // Each file has 2 function nodes
        assert!(multi.graph.node_count() >= 6);

        // Verify all files are in node_locations
        for (_, file_path) in &multi.node_locations {
            assert!(multi.file_nodes.contains(file_path));
        }
    }

    #[test]
    fn test_same_name_different_files() {
        // Both module_a.py and module_b.py have a function named "helper"
        // They should have different NodeIds and both be present
        let files = vec![
            Path::new("tests/test-fixtures/multi-file-project/module_a.py"),
            Path::new("tests/test-fixtures/multi-file-project/module_b.py"),
        ];
        let multi = parse_files(&files).unwrap();

        // Find all helper functions
        let helpers: Vec<_> = multi.graph.nodes()
            .filter(|n| n.name == "helper")
            .collect();

        // Should have 2 helpers with different IDs
        assert_eq!(helpers.len(), 2);

        // Their IDs should be different (file_path::symbol_name format)
        assert_ne!(helpers[0].id, helpers[1].id);
    }

    #[test]
    fn test_node_locations_mapping() {
        let files = vec![
            Path::new("tests/test-fixtures/multi-file-project/module_a.py"),
            Path::new("tests/test-fixtures/multi-file-project/main.py"),
        ];
        let multi = parse_files(&files).unwrap();

        // Verify node_locations maps each node to a file in file_nodes
        for (node_id, file_path) in &multi.node_locations {
            assert!(multi.file_nodes.contains(file_path),
                "Node {} maps to file {:?} which is not in file_nodes", node_id, file_path);
        }
    }

    #[test]
    fn test_file_nodes_reverse_lookup() {
        let files = vec![
            Path::new("tests/test-fixtures/multi-file-project/module_a.py"),
            Path::new("tests/test-fixtures/multi-file-project/module_b.py"),
        ];
        let multi = parse_files(&files).unwrap();

        // Verify each file has a file node entry
        // Note: file_nodes stores canonicalized absolute paths
        assert_eq!(multi.file_nodes.len(), 2);

        // Verify the paths in file_nodes match our input files
        for file_path in &multi.file_nodes {
            assert!(file_path.ends_with("module_a.py") || file_path.ends_with("module_b.py"));
        }
    }

    #[test]
    fn test_deterministic_merging() {
        let file_a = Path::new("tests/test-fixtures/multi-file-project/module_a.py");
        let file_b = Path::new("tests/test-fixtures/multi-file-project/module_b.py");

        // Parse in different orders
        let multi1 = parse_files(&[file_a, file_b]).unwrap();
        let multi2 = parse_files(&[file_b, file_a]).unwrap();

        // Node counts should be the same
        assert_eq!(multi1.graph.node_count(), multi2.graph.node_count());
        assert_eq!(multi1.graph.edge_count(), multi2.graph.edge_count());

        // Node IDs should be the same (order-independent)
        let ids1: Vec<_> = multi1.graph.nodes().map(|n| n.id.clone()).collect();
        let ids2: Vec<_> = multi2.graph.nodes().map(|n| n.id.clone()).collect();
        assert_eq!(ids1, ids2);
    }

    #[test]
    fn test_edge_preservation() {
        let files = vec![
            Path::new("tests/test-fixtures/multi-file-project/module_a.py"),
        ];
        let multi = parse_files(&files).unwrap();

        // module_a.py has: process() -> helper()
        // Should have at least 1 Calls edge
        let calls_count = multi.graph.edges()
            .filter(|e| e.edge_type == crate::graph::EdgeType::Calls)
            .count();

        assert!(calls_count >= 1, "Should have at least 1 Calls edge");
    }

    #[test]
    fn test_parse_directory() {
        let root = Path::new("tests/test-fixtures/multi-file-project");
        let multi = parse_directory(root).unwrap();

        // Should find all 3 Python files
        assert_eq!(multi.file_nodes.len(), 3);
        assert!(multi.graph.node_count() >= 6);
    }

    #[test]
    fn test_empty_file_list() {
        let files: Vec<&Path> = vec![];
        let multi = parse_files(&files).unwrap();

        assert_eq!(multi.graph.node_count(), 0);
        assert_eq!(multi.file_nodes.len(), 0);
        assert!(multi.node_locations.is_empty());
    }

    #[test]
    fn test_multifilegraph_new() {
        let multi = MultiFileGraph::new();

        assert_eq!(multi.graph.node_count(), 0);
        assert_eq!(multi.file_nodes.len(), 0);
        assert!(multi.node_locations.is_empty());
    }

    #[test]
    fn test_multifilegraph_default() {
        let multi = MultiFileGraph::default();

        assert_eq!(multi.graph.node_count(), 0);
        assert_eq!(multi.file_nodes.len(), 0);
        assert!(multi.node_locations.is_empty());
    }
}
