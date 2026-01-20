//! Python parser using tree-sitter
//!
//! This module parses Python source files and extracts top-level
//! functions and classes into graph nodes.

use tree_sitter::{Parser as TsParser};
use tree_sitter_python::LANGUAGE;
use crate::graph::{Edge, EdgeType, Graph, Node, NodeType};
use std::collections::HashMap;
use std::path::Path;
use petgraph::stable_graph::NodeIndex;

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

    // 6. Build graph with nodes
    let mut graph = Graph::new();
    let mut node_map: HashMap<(std::path::PathBuf, String), NodeIndex> = HashMap::new();

    for node in nodes {
        // Clone the fields we need for the key before moving node
        let file_path = node.file_path.clone();
        let name = node.name.clone();
        let idx = graph.add_node(node);
        // Use (file_path, name) as key for file-scoped resolution
        // Use .entry().or_insert() to keep the FIRST definition for duplicate names
        node_map.entry((file_path, name)).or_insert(idx);
    }

    // 7. Extract and add calls edges
    let edges = extract_calls_edges(&root_node, &canonical_path, source_bytes, &node_map);
    for (from, to) in edges {
        graph.add_edge(from, to, Edge { edge_type: EdgeType::Calls });
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

/// Extract calls edges from the syntax tree
///
/// Walks the AST to find `call` nodes and creates edges between
/// caller and callee functions. Only creates edges within the same file
/// using file-scoped resolution.
fn extract_calls_edges(
    root_node: &tree_sitter::Node,
    file_path: &Path,
    source: &[u8],
    node_map: &HashMap<(std::path::PathBuf, String), NodeIndex>,
) -> Vec<(NodeIndex, NodeIndex)> {
    let mut edges = Vec::new();
    let mut cursor = root_node.walk();
    // Create PathBuf once for cheaper clone() in loop (avoid repeated to_path_buf())
    let file_path_buf = file_path.to_path_buf();

    // Walk the entire tree using tree-sitter's cursor traversal
    loop {
        let node = cursor.node();

        if node.kind() == "call" {
            // Extract the function name being called
            if let Some(callee_name) = extract_call_name(&node, source) {
                // Find the parent function_definition (caller)
                if let Some(caller_idx) = find_parent_function(&node, root_node, source, &file_path_buf, node_map) {
                    // Look up the callee in the node map (same file only)
                    let key = (file_path_buf.clone(), callee_name);
                    if let Some(&callee_idx) = node_map.get(&key) {
                        edges.push((caller_idx, callee_idx));
                    }
                    // Unresolved calls are silently skipped (best-effort)
                }
            }
        }

        // Depth-first traversal: try children first, then siblings, then go up
        if cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            continue;
        }
        // No more children or siblings at this level, go up
        loop {
            if !cursor.goto_parent() {
                // Reached the root, we're done
                return edges;
            }
            if cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// Extract the function name from a call node
///
/// For simple calls like `foo()`, extracts "foo".
/// For dotted calls like `obj.method()` or `module.func()`,
/// extracts the full dotted name (which likely won't resolve).
fn extract_call_name(call_node: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    // The function being called is typically the first child
    call_node.child(0)
        .and_then(|child| match child.kind() {
            // Simple identifier: foo()
            "identifier" => child.utf8_text(source).ok().map(|s| s.to_string()),
            // Dotted/attribute access: obj.method() or module.func()
            // We extract the full dotted name, which likely won't resolve to a node
            "attribute" | "call" => extract_full_call_name(&child, source),
            _ => None,
        })
}

/// Helper to extract full dotted call names recursively
fn extract_full_call_name(node: &tree_sitter::Node, source: &[u8]) -> Option<String> {
    // For attribute nodes like "module.func", we want to extract "module.func"
    // For nested call nodes, we recursively build the name
    match node.kind() {
        "identifier" => node.utf8_text(source).ok().map(|s| s.to_string()),
        "attribute" => {
            // attribute has "object" and "attribute" fields
            // e.g., for "os.path.exists", this is:
            //   (attribute (attribute (identifier) "os") (identifier) "path") (identifier) "exists")
            // We want to build "os.path.exists"
            let object = node.child_by_field_name("object");
            let attr = node.child_by_field_name("attribute");
            match (object, attr) {
                (Some(obj), Some(atr)) => {
                    let obj_name = extract_full_call_name(&obj, source);
                    let attr_name = atr.utf8_text(source).ok().map(|s| s.to_string());
                    match (obj_name, attr_name) {
                        (Some(o), Some(a)) => Some(format!("{}.{}", o, a)),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Find the parent function_definition containing a node
///
/// Walks up the tree to find the enclosing function_definition.
/// Returns None if the call is not inside a function (e.g., top-level module code).
fn find_parent_function(
    node: &tree_sitter::Node,
    root_node: &tree_sitter::Node,
    source: &[u8],
    file_path: &std::path::PathBuf,  // Changed from &Path to &PathBuf for cheaper clone()
    node_map: &HashMap<(std::path::PathBuf, String), NodeIndex>,
) -> Option<NodeIndex> {
    let mut current = *node;

    // Walk up the tree
    loop {
        // Try to get the parent
        if let Some(parent) = current.parent() {
            current = parent;

            if current.kind() == "function_definition" {
                // Found the enclosing function, extract its name
                if let Some(func_name) = extract_node_name(&current, source) {
                    let key = (file_path.clone(), func_name);
                    return node_map.get(&key).copied();
                }
            }

            // Stop if we've reached the root
            if current == *root_node {
                break;
            }
        } else {
            break;
        }
    }

    None
}

#[cfg(test)]
mod tests {

    use crate::graph::EdgeType;
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

    #[test]
    fn test_extract_calls_edges() {
        let parser = crate::parser::Parser::new();
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
        let parser = crate::parser::Parser::new();
        let graph = parser.parse_file(
            Path::new("tests/test-fixtures/calls_with_unresolved.py"),
            &Language::Python,
        ).unwrap();

        // Should have 2 nodes (my_func, helper)
        assert_eq!(graph.node_count(), 2);

        // Should have 1 edge (my_func→helper), unresolved calls skipped
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_no_calls_no_edges() {
        // Verify that functions without calls create no edges
        let parser = crate::parser::Parser::new();
        let graph = parser.parse_file(
            Path::new("tests/test-fixtures/sample.py"),
            &Language::Python,
        ).unwrap();

        // sample.py has no function calls
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_edge_case_duplicate_functions() {
        // Verify behavior with duplicate function names
        let parser = crate::parser::Parser::new();
        let graph = parser.parse_file(
            Path::new("tests/test-fixtures/calls_edge_cases.py"),
            &Language::Python,
        ).unwrap();

        // Should have 5 top-level nodes: helper (2x), caller, dotted_caller, method_caller
        assert_eq!(graph.node_count(), 5);

        // Should have 1 edge (caller→first helper)
        assert_eq!(graph.edge_count(), 1);

        // Verify the edge is from caller to a helper
        let mut found_caller_to_helper = false;
        for (from, to, edge) in graph.edge_endpoints() {
            if let (Some(from_node), Some(to_node)) = (graph.node_weight(from), graph.node_weight(to)) {
                if from_node.name == "caller" && to_node.name == "helper" {
                    assert_eq!(edge.edge_type, EdgeType::Calls);
                    found_caller_to_helper = true;
                }
            }
        }
        assert!(found_caller_to_helper);
    }
}
