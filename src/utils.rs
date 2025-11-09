//! Utility functions for path manipulation and common operations.
//!
//! This module provides helper functions used throughout the Miou bot application
//! for file system operations and path construction.

use std::path::PathBuf;

/// Constructs a file system path by joining a directory path with a subdirectory.
///
/// This is a convenience function that combines path components and returns a
/// platform-independent path string. It handles the path separator automatically
/// based on the operating system.
///
/// # Arguments
///
/// * `dir_path` - The base directory path
/// * `subdir_path` - The subdirectory or file name to append
///
/// # Returns
///
/// A `String` containing the joined path.
///
/// # Panics
///
/// Panics if the resulting path contains invalid UTF-8 characters.
///
/// # Examples
///
/// ```
/// # use miou::utils::get_path;
/// let path = get_path("/home/user", "config");
/// assert_eq!(path, "/home/user/config");
/// ```
pub fn get_path(dir_path: &str, subdir_path: &str) -> String {
    let sqlite_path_buf: PathBuf = [dir_path, subdir_path].iter().collect();
    sqlite_path_buf.to_str().unwrap().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_path_simple() {
        let path = get_path("/home/user", "config");
        #[cfg(unix)]
        assert_eq!(path, "/home/user/config");
        #[cfg(windows)]
        assert_eq!(path, "\\home\\user\\config");
    }

    #[test]
    fn test_get_path_with_file() {
        let path = get_path("/var/data", "alerts.json");
        #[cfg(unix)]
        assert_eq!(path, "/var/data/alerts.json");
        #[cfg(windows)]
        assert_eq!(path, "\\var\\data\\alerts.json");
    }

    #[test]
    fn test_get_path_relative_paths() {
        let path = get_path(".", "data");
        #[cfg(unix)]
        assert_eq!(path, "./data");
        #[cfg(windows)]
        assert_eq!(path, ".\\data");
    }

    #[test]
    fn test_get_path_empty_subdir() {
        let path = get_path("/home/user", "");
        #[cfg(unix)]
        assert_eq!(path, "/home/user/");
        #[cfg(windows)]
        assert_eq!(path, "\\home\\user\\");
    }

    #[test]
    fn test_get_path_nested_subdirs() {
        let path = get_path("/home/user", "config/settings");
        #[cfg(unix)]
        assert_eq!(path, "/home/user/config/settings");
        #[cfg(windows)]
        assert_eq!(path, "\\home\\user\\config/settings");
    }

    #[test]
    fn test_get_path_current_dir() {
        let path = get_path(".", "config.toml");
        #[cfg(unix)]
        assert_eq!(path, "./config.toml");
        #[cfg(windows)]
        assert_eq!(path, ".\\config.toml");
    }

    #[test]
    fn test_get_path_parent_dir() {
        let path = get_path("..", "data");
        #[cfg(unix)]
        assert_eq!(path, "../data");
        #[cfg(windows)]
        assert_eq!(path, "..\\data");
    }

    #[test]
    fn test_get_path_with_spaces() {
        let path = get_path("/home/my folder", "my file.txt");
        #[cfg(unix)]
        assert_eq!(path, "/home/my folder/my file.txt");
        #[cfg(windows)]
        assert_eq!(path, "\\home\\my folder\\my file.txt");
    }

    #[test]
    fn test_get_path_multiple_components() {
        let base = get_path("/home", "user");
        let final_path = get_path(&base, "config");
        #[cfg(unix)]
        assert_eq!(final_path, "/home/user/config");
        #[cfg(windows)]
        assert_eq!(final_path, "\\home\\user\\config");
    }
}
