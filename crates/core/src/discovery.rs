//! File discovery module with gitignore-aware filtering
//!
//! This module provides utilities for discovering files in a project directory
//! while respecting .gitignore patterns. The API is generic and works with any
//! language via glob patterns.

use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Discover files matching glob patterns in a project directory
///
/// # Arguments
/// * `root` - Root directory to search
/// * `patterns` - Glob patterns (e.g., &["**/*.py", "src/**/*.rs"])
///
/// # Returns
/// Vector of absolute paths to matching files, excluding those matched by .gitignore
///
/// # Example
/// ```no_run
/// use graph_migrator_core::discovery;
///
/// // Find all Python files
/// let files = discovery::discover_files(std::path::Path::new("my_project"), &["**/*.py"]);
/// println!("Found {} files", files.len());
///
/// // Find Python files in specific directories
/// let src_tests = discovery::discover_files(std::path::Path::new("my_project"), &["src/**/*.py", "tests/**/*.py"]);
/// ```
pub fn discover_files(root: &Path, patterns: &[&str]) -> Vec<PathBuf> {
    // Canonicalize root upfront to ensure all returned paths are absolute
    // If root doesn't exist or can't be canonicalized, return empty vec
    let canonical_root = match root.canonicalize() {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };

    let mut files = Vec::new();

    // Build a glob set from the provided patterns for efficient matching
    let glob_matcher = match build_glob_matcher(patterns) {
        Ok(matcher) => matcher,
        Err(_) => {
            // If glob patterns are invalid, return empty results
            return Vec::new();
        }
    };

    // Use WalkBuilder for idiomatic gitignore-aware traversal
    let walker = build_walker(&canonical_root);

    for result in walker {
        match result {
            Ok(entry) => {
                // Skip directories - we only want files
                if let Some(ft) = entry.file_type() {
                    if ft.is_file() {
                        // Get the path relative to canonical_root for glob matching
                        if let Ok(rel_path) = entry.path().strip_prefix(&canonical_root) {
                            // Check if the file matches any of our patterns
                            if glob_matcher.is_match(rel_path) {
                                // WalkBuilder already gives us absolute paths
                                files.push(entry.into_path());
                            }
                        }
                    }
                }
            }
            Err(err) => {
                // Log walk errors but continue processing other files
                eprintln!("Warning: Error walking directory: {}", err);
            }
        }
    }

    files
}

/// Discover Python files in a project directory (convenience wrapper)
///
/// # Arguments
/// * `root` - Root directory to search
///
/// # Returns
/// Vector of absolute paths to Python files, excluding those matched by .gitignore
///
/// # Example
/// ```no_run
/// use graph_migrator_core::discovery;
///
/// let files = discovery::discover_python_files(std::path::Path::new("my_project"));
/// println!("Found {} Python files", files.len());
/// ```
pub fn discover_python_files(root: &Path) -> Vec<PathBuf> {
    discover_files(root, &["**/*.py"])
}

/// Build a glob matcher from the provided patterns
///
/// This converts the string patterns into a GlobSet for efficient matching.
fn build_glob_matcher(patterns: &[&str]) -> Result<globset::GlobSet, globset::Error> {
    use globset::GlobSetBuilder;

    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(globset::Glob::new(pattern)?);
    }
    builder.build()
}

/// Build a WalkBuilder with proper ignore configuration
fn build_walker(root: &Path) -> ignore::Walk {
    let mut builder = WalkBuilder::new(root);
    builder
        .git_ignore(true)
        .git_exclude(true)
        .hidden(false)
        .parents(true);  // Also check parent directories for .gitignore

    // Explicitly add .gitignore if it exists (needed for test environments
    // where WalkBuilder may not automatically discover it)
    let gitignore_path = root.join(".gitignore");
    if gitignore_path.exists() {
        let _ = builder.add_ignore(gitignore_path);
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_discover_basic() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        File::create(root.join("main.py")).unwrap();
        File::create(root.join("utils.py")).unwrap();

        let files = discover_python_files(root);

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.is_absolute()));
        assert!(files.iter().any(|p| p.ends_with("main.py")));
        assert!(files.iter().any(|p| p.ends_with("utils.py")));
    }

    #[test]
    fn test_respect_gitignore() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create .gitignore
        let mut gitignore = File::create(root.join(".gitignore")).unwrap();
        gitignore.write_all(b"venv/\n*.pyc\n").unwrap();

        // Create files
        fs::create_dir_all(root.join("venv")).unwrap();
        File::create(root.join("venv/lib.py")).unwrap();
        File::create(root.join("main.py")).unwrap();
        File::create(root.join("main.pyc")).unwrap();

        let files = discover_python_files(root);

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("main.py"));
        assert!(!files.iter().any(|p| p.to_string_lossy().contains("venv")));
    }

    #[test]
    fn test_custom_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create files in different directories
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        File::create(root.join("src/main.py")).unwrap();
        File::create(root.join("tests/test_main.py")).unwrap();
        File::create(root.join("setup.py")).unwrap();

        // Only discover src and tests
        let files = discover_files(root, &["src/**/*.py", "tests/**/*.py"]);

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.is_absolute()));
        assert!(files.iter().any(|p| p.to_string_lossy().contains("src/")));
        assert!(files.iter().any(|p| p.to_string_lossy().contains("tests/")));
    }

    #[test]
    fn test_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let files = discover_python_files(root);

        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::create_dir_all(root.join("pkg/subpkg")).unwrap();
        File::create(root.join("pkg/mod.py")).unwrap();
        File::create(root.join("pkg/subpkg/mod.py")).unwrap();

        let files = discover_python_files(root);

        assert_eq!(files.len(), 2);
        assert!(files.iter().all(|p| p.is_absolute()));
    }

    #[test]
    fn test_absolute_paths_contract() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        File::create(root.join("test.py")).unwrap();

        let files = discover_python_files(root);

        assert_eq!(files.len(), 1);
        assert!(files[0].is_absolute(), "All paths should be absolute");
    }
}
