//! Common test utilities

use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Create a temporary directory with a rusk.yml file
pub fn create_test_config(content: &str) -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("rusk.yml");
    fs::write(&config_path, content).unwrap();
    (temp_dir, config_path)
}

/// Create a test config in a subdirectory
pub fn create_test_config_in_subdir(content: &str) -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("rusk.yml");
    let sub_dir = temp_dir.path().join("subdir");

    fs::write(&config_path, content).unwrap();
    fs::create_dir(&sub_dir).unwrap();

    (temp_dir, config_path, sub_dir)
}
