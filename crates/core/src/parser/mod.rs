//! Parser module for extracting code structure from source files
//!
//! This module provides language-specific parsers using tree-sitter
//! to build dependency graphs from source code.

use std::path::Path;

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
