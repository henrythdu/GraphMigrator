//! GraphMigrator Core Library
//!
//! This library provides the core data structures and functionality for
//! building and querying dependency graphs from source code.

pub mod discovery;
pub mod graph;
pub mod parser;
pub mod queries;

// Re-export commonly used types
pub use graph::{Edge, Graph, Node, NodeType};
