# Epic 4: File Discovery

**Date**: 2025-01-21
**Status**: Draft
**Phase**: Phase 1 (MVP)

## Goal

Add gitignore-aware file discovery to find Python files in a project directory. This is the foundation for multi-file parsing.

> **Positioning Statement**: This epic focuses solely on discovering files, not parsing them. It provides the API that future epics will use to get file lists for parsing.

## Scope

### What Epic 4 Does

- Discover files using glob patterns
- Respect `.gitignore` patterns (exclude virtual environments, cache directories)
- Return absolute paths to all discovered files
- Support custom glob patterns for filtering
- **Generic API** designed for multi-language support (future-proofing)

### What Epic 4 Does NOT Do

- **No parsing** - that's Epic 5
- **No import extraction** - that's Epic 6
- **No import resolution** - that's Epic 7
- **No graph building** - that's Epic 5
- **No language-specific logic** - keeps API generic

### Scope Constraints

- Gitignore-aware filtering only
- Glob pattern matching via the `glob` crate
- Simple API: return `Vec<PathBuf>`
- **Designed for extensibility** to other languages (future)

## Architecture

### API Surface

```rust
// crates/core/src/discovery.rs

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
/// let files = discovery::discover_files(Path::new("my_project"), &["**/*.py"]);
/// println!("Found {} files", files.len());
///
/// // Find Python files in specific directories
/// let src_tests = discovery::discover_files(Path::new("my_project"), &["src/**/*.py", "tests/**/*.py"]);
/// ```
pub fn discover_files(root: &Path, patterns: &[&str]) -> Vec<PathBuf> {
    // Implementation...
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
/// let files = discovery::discover_python_files(Path::new("my_project"));
/// println!("Found {} Python files", files.len());
/// ```
pub fn discover_python_files(root: &Path) -> Vec<PathBuf> {
    discover_files(root, &["**/*.py"])
}
```

### Implementation

```rust
use glob::glob;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::{Path, PathBuf};

pub fn discover_files(root: &Path, patterns: &[&str]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Load .gitignore if present
    let gitignore = load_gitignore(root);

    for pattern in patterns {
        let full_pattern = root.join(pattern).to_string_lossy().to_string();

        if let Ok(entries) = glob(&full_pattern) {
            for entry in entries.flatten() {
                // Only include files (not directories)
                if !entry.is_file() {
                    continue;
                }

                // Skip if matched by gitignore
                if let Some(rel_path) = entry.strip_prefix(root).ok() {
                    if gitignore.matched(rel_path, false).is_ignore() {
                        continue;
                    }
                }

                files.push(entry);
            }
        }
    }

    files
}

/// Convenience wrapper for Python files
pub fn discover_python_files(root: &Path) -> Vec<PathBuf> {
    discover_files(root, &["**/*.py"])
}
    let mut files = Vec::new();

    // Load .gitignore if present
    let gitignore = load_gitignore(root);

    for pattern in patterns {
        let full_pattern = root.join(pattern).to_string_lossy().to_string();

        if let Ok(entries) = glob(&full_pattern) {
            for entry in entries.flatten() {
                // Only include files (not directories)
                if !entry.is_file() {
                    continue;
                }

                // Skip if matched by gitignore
                if let Some(rel_path) = entry.strip_prefix(root).ok() {
                    if gitignore.matched(rel_path, false).is_ignore() {
                        continue;
                    }
                }

                files.push(entry);
            }
        }
    }

    files
}

fn load_gitignore(root: &Path) -> Gitignore {
    let mut builder = GitignoreBuilder::new(root);

    let gitignore_path = root.join(".gitignore");
    if gitignore_path.exists() {
        let _ = builder.add(gitignore_path);
    }

    builder.build().unwrap_or_else(|_| {
        // Fallback: empty gitignore that matches nothing
        GitignoreBuilder::new(root).build().unwrap()
    })
}
```

**Dependencies:**
- Add `glob = "0.3"` to `Cargo.toml`
- Add `ignore = "0.4"` to `Cargo.toml`

## Future Extensibility

The generic `discover_files()` API is designed to support future enhancements:

### Multi-Language Support

The pattern-based API works for any language:

```rust
// Rust files
let rust_files = discover_files(root, &["**/*.rs"]);

// JavaScript/TypeScript
let js_files = discover_files(root, &["**/*.js", "**/*.ts", "**/*.tsx"]);

// Mixed-language projects
let code_files = discover_files(root, &["**/*.py", "**/*.rs", "**/*.go"]);
```

### Potential Extensions (YAGNI - Not Implementing Yet)

**Incremental discovery**: Track file mtime to return only changed files since last scan.

**Monorepo support**: Add per-package gitignore handling (e.g., `.gitignore` in each package/).

**Symlink handling**: Follow or skip symlinks based on configuration (currently skipped by glob).

**Custom ignore patterns**: Allow passing additional ignore patterns beyond `.gitignore`.

## Testing Strategy

### Unit Tests

```rust
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
    }
}
```

### Test Fixture

```
tests/test-fixtures/discovery-project/
├── src/
│   └── main.py
├── tests/
│   └── test_main.py
├── venv/
│   └── lib.py
├── .gitignore       # contains "venv/"
└── setup.py
```

**Expected result:**
- Finds: `src/main.py`, `tests/test_main.py`, `setup.py`
- Excludes: `venv/lib.py` (matched by gitignore)

## Acceptance Criteria

Epic 4 is complete when:

- [ ] `discover_python_files()` returns all `.py` files in a directory
- [ ] `.gitignore` patterns are respected (e.g., `venv/` is excluded)
- [ ] `discover_files()` supports custom glob patterns for any language
- [ ] `discover_python_files()` is implemented as a convenience wrapper
- [ ] All unit tests pass
- [ ] Integration test with fixture passes
- [ ] API is documented with examples
- [ ] API is generic and language-agnostic

## Dependencies

Add to `crates/core/Cargo.toml`:

```toml
[dependencies]
glob = "0.3"
ignore = "0.4"

[dev-dependencies]
tempfile = "3"
```

## What Success Looks Like

After Epic 4, we can run:

```rust
// Simple case: all Python files
let files = discover_python_files(Path::new("my_project"));

assert_eq!(files.len(), 15);
assert!(files.iter().all(|p| p.extension() == Some("py".as_ref())));
assert!(!files.iter().any(|p| p.to_string_lossy().contains("venv")));

// Custom patterns: specific directories
let src_and_tests = discover_files(Path::new("my_project"), &["src/**/*.py", "tests/**/*.py"]);

// Future: multi-language projects
let all_code = discover_files(Path::new("mixed_project"), &["**/*.py", "**/*.rs", "**/*.js"]);
```

This provides the foundation for Epic 5 (multi-file parsing) to consume.

## Next Epic

**Epic 5: Multi-File Parsing (Graph Merging)**

*Goal*: Parse multiple files and merge their graphs into a unified structure.

*Rough scope*:
- Use `discover_python_files()` to get file list
- Parse each file using existing `parse_file()`
- Merge graphs without duplicate nodes
- Return unified `Graph`
