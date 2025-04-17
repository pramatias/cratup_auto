// use anyhow::Result;
use colored::Colorize;
use log::debug;

/// Holds the split parts of a file path.
#[derive(Debug, PartialEq, Eq)]
pub struct FileParts {
    pub prefix: String,
    pub parent: String,
    pub file: String,
}

/// Directory type with file parts.
#[derive(Debug, PartialEq, Eq)]
pub enum DirectoryType {
    Start(FileParts),
    Nested(FileParts),
}

/// Splits the given file path string relative to the current directory string.
///
/// # Parameters
/// - `file_path_str`: The full file path as a string.
/// - `current_dir_str`: The current directory as a string.
///
/// # Behavior
/// - If `file_path_str` starts with `current_dir_str`, that portion is removed before further processing.
/// - The remaining path is split by the `/` character into components.
/// - Depending on the number of components, a `DirectoryType` is constructed:
///   - **0 components:** Returns a `DirectoryType::Start` with empty parts.
///   - **1 component:** Treats it as a file in the start directory.
///   - **2 components:** Treats it as a parent directory plus file.
///   - **More than 2 components:** Uses the first components (if any) as a prefix, the second-to-last as parent, and the last component as file.
///
pub fn split_dir_path_parts_str(file_path_str: &str, current_dir_str: &str) -> DirectoryType {
    // Determine the relative path by attempting to remove the current_dir prefix.
    let relative = if file_path_str.starts_with(current_dir_str) {
        // Remove the prefix. If the prefix is not empty and is immediately followed by a separator,
        // also remove the separator.
        let mut rel = &file_path_str[current_dir_str.len()..];
        if rel.starts_with('/') {
            rel = &rel[1..];
        }
        rel
    } else {
        file_path_str
    };

    // Split the relative path by '/' and filter out any empty components
    let components: Vec<String> = relative
        .split('/')
        .filter(|c| !c.is_empty())
        .map(|c| {
            // Here you can insert logging if needed, e.g., `debug!("Processing component: {}", c);`
            c.to_string()
        })
        .collect();

    // Match based on the number of components
    match components.len() {
        0 => {
            // In case there is no remaining information in the path.
            let parts = FileParts {
                prefix: String::new(),
                parent: String::new(),
                file: String::new(),
            };
            DirectoryType::Start(parts)
        }
        1 => {
            // Only a single component means it is a file in the start directory.
            let parts = FileParts {
                prefix: String::new(),
                parent: String::new(),
                file: components[0].clone(),
            };
            DirectoryType::Start(parts)
        }
        2 => {
            // Two components: first is parent directory, second is file.
            let parts = FileParts {
                prefix: String::new(),
                parent: components[0].clone(),
                file: components[1].clone(),
            };
            DirectoryType::Nested(parts)
        }
        _ => {
            // More than two components: the last component is the file,
            // the second-to-last is the parent, and any preceding are joined as a prefix.
            let file = components.last().unwrap().clone();
            let parent = components[components.len() - 2].clone();
            let prefix = components[..components.len() - 2].join("/");
            let parts = FileParts {
                prefix,
                parent,
                file,
            };
            DirectoryType::Nested(parts)
        }
    }
}

/// Constructs the display path based on the directory type.
/// It now accepts string slices (the string representations of the paths)
/// instead of `Path` objects.
pub fn build_directory_display(file_path: &str, current_dir: &str) -> String {
    debug!("File path details: {}", file_path);
    debug!("Current directory: {}", current_dir);

    // Get the directory type from the helper function.
    // In your production code, `split_dir_path_parts_str` is assumed to convert the strings
    // into the proper directory parts.
    let dir_type = split_dir_path_parts_str(file_path, current_dir);
    debug!("Constructed directory type: {:?}", dir_type);

    match dir_type {
        DirectoryType::Start(ref parts) => {
            debug!("Processing Start directory type with parts: {:?}", parts);
            if parts.prefix.is_empty() && parts.parent.is_empty() {
                debug!("Simple file case - no prefix or parent");
                format!("{}/{}", ".".green(), parts.file)
            } else if !parts.prefix.is_empty() {
                debug!("Prefix present: {}", parts.prefix);
                format!("{}/{}/{}", parts.prefix, parts.parent.green(), parts.file)
            } else {
                debug!("Only file name present");
                format!("{}/{}", ".".green(), parts.file)
            }
        }
        DirectoryType::Nested(ref parts) => {
            debug!("Processing Nested directory type with parts: {:?}", parts);
            let colored_parent = parts.parent.green();
            if parts.prefix.is_empty() {
                debug!("Nested path without prefix");
                format!("{}/{}", colored_parent, parts.file)
            } else {
                debug!("Nested path with prefix: {}", parts.prefix);
                format!("{}/{}{}/{}", parts.prefix, "", colored_parent, parts.file)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_relative_path() {
        // When the file path equals the current directory, we get an empty relative path.
        let result = split_dir_path_parts_str("some/path", "some/path");
        let expected = DirectoryType::Start(FileParts {
            prefix: String::new(),
            parent: String::new(),
            file: String::new(),
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_single_component() {
        let result = split_dir_path_parts_str("some/path/file.txt", "some/path");
        let expected = DirectoryType::Start(FileParts {
            prefix: String::new(),
            parent: String::new(),
            file: "file.txt".to_string(),
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_two_components() {
        let result = split_dir_path_parts_str("some/path/dir/file.txt", "some/path");
        let expected = DirectoryType::Nested(FileParts {
            prefix: String::new(),
            parent: "dir".to_string(),
            file: "file.txt".to_string(),
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_deeply_nested_path() {
        let result = split_dir_path_parts_str("base/dir/subdir1/subdir2/file.txt", "base/dir");
        let expected = DirectoryType::Nested(FileParts {
            // prefix should join all components except the last two
            prefix: "subdir1".to_string(),
            parent: "subdir2".to_string(),
            file: "file.txt".to_string(),
        });
        assert_eq!(result, expected);
    }

    #[test]
    fn test_non_matching_prefix() {
        // If the file path does not start with the current directory,
        // the function should use the full file path.
        let result = split_dir_path_parts_str("different/path/file.txt", "base/dir");
        // Here the relative path is "different/path/file.txt" which splits into 3 components:
        // "different", "path", "file.txt"
        let expected = DirectoryType::Nested(FileParts {
            prefix: "different".to_string(),
            parent: "path".to_string(),
            file: "file.txt".to_string(),
        });
        assert_eq!(result, expected);
    }
}
