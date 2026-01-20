//! Python parser using tree-sitter
//!
//! This module parses Python source files and extracts top-level
//! functions and classes into graph nodes.

use tree_sitter::{Parser as TsParser};
use tree_sitter_python::LANGUAGE;
use crate::graph::{Graph, Node, NodeType};
use std::path::Path;

/// Parse a Python source file and extract its structure
///
/// # Arguments
/// * `path` - Path to the Python file to parse
///
/// # Returns
/// A `Graph` containing nodes for extracted functions and classes
pub fn parse_file(path: &Path) -> anyhow::Result<Graph> {
    // 1. Canonicalize path for stable node IDs (prevents duplicate IDs from relative/absolute paths)
    let canonical_path = std::fs::canonicalize(path)?;

    // 2. Read file contents to String
    let source = std::fs::read_to_string(&canonical_path)?;

    // 3. Create tree-sitter parser
    let mut parser = TsParser::new();
    parser.set_language(&LANGUAGE.into())?;

    // 4. Parse source code
    let tree = parser.parse(&source, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse Python file: {}", canonical_path.display()))?;

    // 5. Extract top-level nodes only (functions and classes)
    let root_node = tree.root_node();
    let source_bytes = source.as_bytes();
    let nodes = extract_top_level_nodes(&root_node, &canonical_path, source_bytes);

    // 6. Build graph
    let mut graph = Graph::new();
    for node in nodes {
        graph.add_node(node);
    }

    Ok(graph)
}

/// Extract top-level function and class definitions from the syntax tree
///
/// Only iterates over direct children of the root node, ensuring we only
/// extract top-level definitions and not nested functions/classes.
fn extract_top_level_nodes(root_node: &tree_sitter::Node, file_path: &Path, source: &[u8]) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut cursor = root_node.walk();

    // Only iterate over direct children of root (top-level statements)
    for node in root_node.children(&mut cursor) {
        let (node_type_opt, name_opt) = match node.kind() {
            "function_definition" => (Some(NodeType::Function), extract_node_name(&node, source)),
            "class_definition" => (Some(NodeType::Class), extract_node_name(&node, source)),
            _ => (None, None),
        };

        if let (Some(node_type), Some(name)) = (node_type_opt, name_opt) {
            nodes.push(Node {
                id: format!("{}::{}", file_path.display(), name),
                name,
                node_type,
                language: "python".to_string(),
                file_path: file_path.to_path_buf(),
                line_range: None,
            });
        }
    }

    nodes
}

/// Extract the name from a function_definition or class_definition node
///
/// Uses tree-sitter's named field API to robustly extract the "name" field.
fn extract_node_name(node: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|name_node| name_node.utf8_text(source).ok())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use crate::parser::Language;
    use std::path::Path;

    #[test]
    fn test_parse_python_file() {
        let parser = crate::parser::Parser::new();
        let graph = parser.parse_file(
            Path::new("tests/test-fixtures/sample.py"),
            &Language::Python,
        ).unwrap();

        // Should extract 2 functions + 1 class = 3 nodes
        assert_eq!(graph.node_count(), 3);

        // Verify nodes have correct properties
        let node_names: Vec<&str> = graph.nodes()
            .map(|n| n.name.as_str())
            .collect();

        assert!(node_names.contains(&"hello_world"));
        assert!(node_names.contains(&"another_function"));
        assert!(node_names.contains(&"Greeter"));

        // Verify language is set
        for node in graph.nodes() {
            assert_eq!(node.language, "python");
        }

        // Verify file path is canonicalized
        for node in graph.nodes() {
            assert!(node.file_path.is_absolute());
        }
    }

    #[test]
    fn test_nested_symbols_not_extracted() {
        let parser = crate::parser::Parser::new();
        let graph = parser.parse_file(
            Path::new("tests/test-fixtures/nested.py"),
            &Language::Python,
        ).unwrap();

        // Should extract 2 top-level symbols (outer_function, OuterClass)
        // inner_function and InnerClass should NOT be extracted
        assert_eq!(graph.node_count(), 2);

        let node_names: Vec<&str> = graph.nodes()
            .map(|n| n.name.as_str())
            .collect();

        assert!(node_names.contains(&"outer_function"));
        assert!(node_names.contains(&"OuterClass"));
        assert!(!node_names.contains(&"inner_function"));
        assert!(!node_names.contains(&"InnerClass"));
    }
}
