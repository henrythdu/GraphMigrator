//! Integration test for file discovery module
//!
//! This test uses the pre-created fixture at `tests/test-fixtures/discovery-project/`
//! to verify gitignore-aware file discovery works correctly.

use graph_migrator_core::discovery;
use std::path::Path;

#[test]
fn test_integration_discovery_fixture() {
    // Get the fixture directory path
    let fixture_path = Path::new("tests/test-fixtures/discovery-project");

    // Ensure the fixture exists
    assert!(fixture_path.exists(), "Fixture directory should exist");

    // Discover Python files in the fixture
    let files = discovery::discover_python_files(fixture_path);

    // Expected: finds src/main.py, tests/test_main.py, setup.py
    // Excludes: venv/lib.py (matched by .gitignore)
    assert_eq!(files.len(), 3, "Should find exactly 3 Python files");

    // Verify the expected files are found
    let file_names: Vec<&str> = files.iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .collect();

    assert!(file_names.contains(&"main.py"), "Should find main.py");
    assert!(file_names.contains(&"test_main.py"), "Should find test_main.py");
    assert!(file_names.contains(&"setup.py"), "Should find setup.py");

    // Verify venv files are excluded
    assert!(!files.iter().any(|p| p.to_string_lossy().contains("venv")),
            "Should exclude files in venv/ directory");

    // Verify all paths are absolute
    assert!(files.iter().all(|p| p.is_absolute()), "All paths should be absolute");
}
