//! Graph data structures for dependency tracking
//!
//! Uses `petgraph::StableGraph` to ensure node indices remain stable
//! even when nodes are removed—critical for migration tracking where
//! nodes transition from Pending → Migrated → Superseded.

use petgraph::stable_graph::StableGraph;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use serde::{Deserialize, Serialize};

/// A node in the dependency graph representing a code element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Stable, language-agnostic identifier
    pub id: String,
    /// Symbol name (e.g., "my_function")
    pub name: String,
    /// Type of code element
    pub node_type: NodeType,
    /// Source language (immutable)
    pub language: String,
    /// File path where this symbol is defined
    pub file_path: std::path::PathBuf,
    /// Line range (start, end) if applicable
    pub line_range: Option<(usize, usize)>,
}

/// Types of code elements that can be represented as nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeType {
    File,
    Module,
    Class,
    Interface,
    Struct,
    Function,
    Method,
    GlobalVariable,
    /// MigrationUnit represents a logical grouping of code being migrated together
    MigrationUnit,
}

/// An edge representing a relationship between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Type of relationship
    pub edge_type: EdgeType,
}

/// Types of relationships between nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeType {
    /// Structural hierarchy: File → Class → Method
    Contains,
    /// Execution dependency: Function → Function
    Calls,
    /// Code dependency: Module/File → Module/File
    Imports,
    /// OOP relationship: Class → Class
    Inherits,
    /// Migration link: Legacy → Target
    MigratedTo,
    /// Groups related migrations: Node → MigrationUnit
    PartOfMigration,
}

/// The dependency graph
///
/// Uses `StableGraph` to ensure node indices remain consistent even as
/// nodes are added/removed during migration tracking.
pub struct Graph {
    /// The underlying stable graph (private to enforce encapsulation)
    inner: StableGraph<Node, Edge>,
}

impl Graph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            inner: StableGraph::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: Node) -> petgraph::stable_graph::NodeIndex {
        self.inner.add_node(node)
    }

    /// Add an edge between two nodes
    pub fn add_edge(
        &mut self,
        from: petgraph::stable_graph::NodeIndex,
        to: petgraph::stable_graph::NodeIndex,
        edge: Edge,
    ) -> petgraph::stable_graph::EdgeIndex {
        self.inner.add_edge(from, to, edge)
    }

    /// Get a node by index
    pub fn node_weight(
        &self,
        index: petgraph::stable_graph::NodeIndex,
    ) -> Option<&Node> {
        self.inner.node_weight(index)
    }

    /// Get an edge by index
    pub fn edge_weight(
        &self,
        index: petgraph::stable_graph::EdgeIndex,
    ) -> Option<&Edge> {
        self.inner.edge_weight(index)
    }

    /// Get the number of nodes in the graph
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    /// Get the number of edges in the graph
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// Iterate over all node weights in the graph
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.inner.node_weights()
    }

    /// Iterate over all edge weights in the graph
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.inner.edge_weights()
    }

    /// Get edge endpoints for testing verification
    ///
    /// Returns an iterator of (from_node_index, to_node_index, edge_weight) tuples
    /// for all edges in the graph. This enables tests to verify edge wiring
    /// by asserting both endpoints and edge type.
    pub fn edge_endpoints(
        &self,
    ) -> impl Iterator<Item = (petgraph::stable_graph::NodeIndex, petgraph::stable_graph::NodeIndex, &Edge)> {
        self.inner
            .edge_references()
            .map(|e| (e.source(), e.target(), e.weight()))
    }

    /// Get all node indices in the graph
    pub fn node_indices(&self) -> impl Iterator<Item = petgraph::stable_graph::NodeIndex> + '_ {
        self.inner.node_indices()
    }

    /// Get all edge indices in the graph
    pub fn edge_indices(&self) -> impl Iterator<Item = petgraph::stable_graph::EdgeIndex> + '_ {
        self.inner.edge_indices()
    }

    /// Get the endpoints of a specific edge
    ///
    /// Returns None if the edge index is invalid
    pub fn edge_endpoints_for(
        &self,
        edge_index: petgraph::stable_graph::EdgeIndex,
    ) -> Option<(petgraph::stable_graph::NodeIndex, petgraph::stable_graph::NodeIndex)> {
        self.inner.edge_endpoints(edge_index)
    }

    /// Find a node by its ID
    ///
    /// Returns the node index if found, None otherwise.
    ///
    /// **Note**: This performs a linear scan over all nodes and has O(N) complexity.
    /// For performance-sensitive code, consider maintaining a separate ID-to-index map.
    pub fn find_node_by_id(&self, id: &str) -> Option<petgraph::stable_graph::NodeIndex> {
        self.node_indices()
            .find(|&idx| self.node_weight(idx).map(|n| n.id.as_str()) == Some(id))
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
