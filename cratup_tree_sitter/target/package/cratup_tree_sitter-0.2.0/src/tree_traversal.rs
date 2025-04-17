use tree_sitter::{Node, Parser, Tree};
// use semver::Version;
use derive_more::Display;
use log::debug;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub enum TomlParserError {
    ParseError,
}

// New struct to hold the package info and dependency info.
#[derive(Debug)]
pub struct PackageAndDepsNodes<'a> {
    pub package: Option<(Node<'a>, PkgInfo)>,
    pub dependencies: HashMap<Node<'a>, DepsInfo>,
}

#[derive(Debug, Clone)]
pub struct PackageAndDeps {
    pub package: Option<PkgInfo>,
    pub dependencies: Vec<DepsInfo>,
}

#[derive(Debug, Display, Clone)]
#[display(
    // "Package {} (pair: {}) version {} (pair: {})",
    "{}",
    // name,
    name_pair,
    // version,
    // version_pair
)]
pub struct PkgInfo {
    pub name: String,
    pub version: String,
    pub name_pair: String,
    pub version_pair: String,
}

#[derive(Debug, Display, Clone)]
#[display(
    // "Dependencies {} (pair: {}) version {} (pair: {})",
    "{}",
    // name,
    name_pair,
    // version,
    // version_pair
)]
pub struct DepsInfo {
    pub name: String,
    pub version: String,
    pub name_pair: String,
    pub version_pair: String,
}

#[derive(Debug)]
pub struct TomlParser<'a> {
    pub source: &'a str,
    pub tree: Tree,
}

impl PackageAndDeps {
    /// Returns the number of valid elements.
    ///
    /// This method adds 1 if `package` is present (Some),
    /// plus the number of dependency entries in `dependencies`.
    /// If neither exists, it returns zero.
    pub fn count(&self) -> usize {
        let pkg_count = if self.package.is_some() { 1 } else { 0 };
        pkg_count + self.dependencies.len()
    }
}

/// find_package_and_deps
impl TomlParser<'_> {
    // Updated function signature and implementation.
    pub fn find_package_and_deps<'b>(&'b self) -> Option<PackageAndDepsNodes<'b>> {
        debug!("Starting to find package and dependencies...");
        let root_node = self.tree.root_node();
        debug!("Root node kind: {}", root_node.kind());

        if root_node.kind() != "document" {
            debug!("Root node is not a document, returning None");
            return None;
        }

        // Collect children into a vector so that the borrow on `root_node` ends.
        debug!("Collecting root node children...");
        let children: Vec<_> = {
            let mut cursor = root_node.walk();
            root_node.children(&mut cursor).collect()
        };
        debug!("Found {} root children", children.len());

        let mut package: Option<(Node<'b>, PkgInfo)> = None;
        let mut dependencies: HashMap<Node<'b>, DepsInfo> = HashMap::new();

        children.iter().enumerate().for_each(|(i, child)| {
            debug!("\nProcessing child {} of kind: {}", i, child.kind());

            if child.kind() == "table" {
                debug!("Found table node");

                // Check for dependencies using an iterator to insert each found dependency.
                if let Some(deps_in_table) = self.find_deps_in_table(*child) {
                    debug!("Found {} dependencies:", deps_in_table.len());
                    deps_in_table.into_iter().for_each(|(node, deps_info)| {
                        dependencies.insert(node, deps_info);
                    });
                } else {
                    debug!("No dependencies found in this table");
                }

                // Check for package info.
                debug!("Looking for package info in table...");
                if let Some(pkg_tuple) = self.find_package_in_table(*child) {
                    package = Some(pkg_tuple);
                } else {
                    debug!("No package info found in this table");
                }
            } else {
                debug!("Skipping non-table node");
            }
        });

        if let Some(pkg) = package {
            Some(PackageAndDepsNodes {
                package: Some(pkg),
                dependencies,
            })
        } else {
            debug!("No package information found in any tables, returning None");
            None
        }
    }
}

/// find_package
impl TomlParser<'_> {
    /// Finds and returns only the package information (PkgInfo).
    pub fn find_package(&self) -> Option<PkgInfo> {
        debug!("Starting to find package...");
        let root_node = self.tree.root_node();
        debug!("Root node kind: {}", root_node.kind());

        if root_node.kind() != "document" {
            debug!("Root node is not a document, returning None");
            return None;
        }

        // Collect children into a vector so that the borrow on `root_node` ends.
        debug!("Collecting root node children...");
        let children: Vec<_> = {
            let mut cursor = root_node.walk();
            root_node.children(&mut cursor).collect()
        };
        debug!("Found {} root children", children.len());

        // Using iterator methods instead of a for loop.
        children
            .iter()
            .enumerate()
            .find_map(|(i, child)| {
                debug!("Processing child {} of kind: {}", i, child.kind());

                if child.kind() == "table" {
                    debug!("Found table node. Looking for package info in table...");
                    if let Some((_node, pkg_info)) = self.find_package_in_table(*child) {
                        debug!("Package info found: {:?}", pkg_info);
                        return Some(pkg_info);
                    } else {
                        debug!("No package info found in this table");
                    }
                } else {
                    debug!("Skipping non-table node");
                }
                None
            })
            .or_else(|| {
                debug!("No package information found in any table, returning None");
                None
            })
    }
}

/// find_deps
#[allow(dead_code)]
impl TomlParser<'_> {
    pub fn find_deps_only<'b>(&'b self) -> Option<HashMap<Node<'b>, DepsInfo>> {
        debug!("Starting to find dependencies only...");
        let root_node = self.tree.root_node();
        debug!("Root node kind: {}", root_node.kind());

        if root_node.kind() != "document" {
            debug!("Root node is not a document, returning None");
            return None;
        }

        // Collect children into a vector so that the borrow on `root_node` ends.
        debug!("Collecting root node children...");
        let children: Vec<_> = {
            let mut cursor = root_node.walk();
            root_node.children(&mut cursor).collect()
        };
        debug!("Found {} root children", children.len());

        let mut dependencies: HashMap<Node<'b>, DepsInfo> = HashMap::new();

        children.iter().enumerate().for_each(|(i, child)| {
            debug!("\nProcessing child {} of kind: {}", i, child.kind());
            if child.kind() == "table" {
                debug!("Found table node");

                self.find_deps_in_table(*child)
                    .map(|deps_in_table| {
                        debug!("Found {} dependencies in table", deps_in_table.len());
                        deps_in_table.into_iter().for_each(|(node, deps_info)| {
                            // Merge or insert dependency information.
                            dependencies.insert(node, deps_info);
                        });
                    })
                    .unwrap_or_else(|| {
                        debug!("No dependencies found in this table");
                    });
            } else {
                debug!("Skipping non-table node");
            }
        });

        if dependencies.is_empty() {
            debug!("No dependency information found in any tables, returning None");
            None
        } else {
            Some(dependencies)
        }
    }
}

/// extract_pkg_info
impl<'a> TomlParser<'a> {
    pub fn extract_pkg_info(&self, table_node: Node<'a>) -> Option<(Node<'a>, PkgInfo)> {
        let strip_quotes = |s: &str| s.replace("\"", "");

        let mut table_cursor = table_node.walk();
        let mut name_opt: Option<String> = None;
        let mut version_opt: Option<String> = None;
        let mut name_pair_opt: Option<String> = None;
        let mut version_pair_opt: Option<String> = None;
        let mut version_node_opt: Option<Node<'a>> = None; // To capture the node where "version" is found

        // Iterate through each child of the table node
        table_node
            .children(&mut table_cursor)
            .filter(|child| child.kind() == "pair")
            .for_each(|table_child| {
                let pair_text = table_child
                    .utf8_text(self.source.as_bytes())
                    .unwrap_or("")
                    .trim()
                    .to_string();

                if let Some(pair_bare_key) = Self::find_child_by_kind(table_child, "bare_key") {
                    let pair_key_text = pair_bare_key
                        .utf8_text(self.source.as_bytes())
                        .unwrap_or("")
                        .trim();

                    // Look for the "name" and "version" keys
                    if pair_key_text == "name" || pair_key_text == "version" {
                        if let Some(string_node) = Self::find_child_by_kind(table_child, "string") {
                            let text = string_node
                                .utf8_text(self.source.as_bytes())
                                .unwrap_or("")
                                .trim()
                                .to_string();
                            if pair_key_text == "name" {
                                name_opt = Some(strip_quotes(&text));
                                name_pair_opt = Some(pair_text);
                            } else if pair_key_text == "version" {
                                version_opt = Some(strip_quotes(&text));
                                version_pair_opt = Some(pair_text);
                                version_node_opt = Some(string_node);
                            }
                        }
                    }
                }
            });

        // Only return if both name and version (and corresponding pair text and version node) were found.
        match (
            version_node_opt,
            name_opt,
            version_opt,
            name_pair_opt,
            version_pair_opt,
        ) {
            (
                Some(version_node),
                Some(name),
                Some(version),
                Some(name_pair),
                Some(version_pair),
            ) => Some((
                version_node,
                PkgInfo {
                    name,
                    version,
                    name_pair,
                    version_pair,
                },
            )),
            _ => None,
        }
    }
}

/// extract_deps_info
impl<'a> TomlParser<'a> {
    /// Helper method to extract the version info from an inline table node.
    fn extract_version_from_inline_table(
        source: &'a str,
        inline_table_node: Node<'a>,
    ) -> Option<(String, String, Node<'a>)> {
        // Closure to strip quotes from strings
        let strip_quotes = |s: &str| s.replace("\"", "");

        inline_table_node
            .children(&mut inline_table_node.walk())
            .filter(|child| child.kind() == "pair")
            .filter_map(|pair_node| {
                // Get the key from the pair
                let key_text = Self::find_child_by_kind(pair_node, "bare_key")
                    .and_then(|node| node.utf8_text(source.as_bytes()).ok())
                    .map(|s| s.trim().to_string())?;

                if key_text == "version" {
                    // Find the string node corresponding to the version value
                    Self::find_child_by_kind(pair_node, "string").and_then(|string_node| {
                        let raw_version = string_node.utf8_text(source.as_bytes()).ok()?.trim();
                        let version = strip_quotes(raw_version);
                        let version_pair_text = pair_node
                            .utf8_text(source.as_bytes())
                            .ok()?
                            .trim()
                            .to_string();
                        Some((version, version_pair_text, string_node))
                    })
                } else {
                    None
                }
            })
            .next() // Return the first matching "version" pair found
    }

    /// extract_deps_info
    pub fn extract_deps_info(&self, table_node: Node<'a>) -> HashMap<Node<'a>, DepsInfo> {
        table_node
            .children(&mut table_node.walk())
            .filter(|pair_node| pair_node.kind() == "pair")
            .filter_map(|pair_node| {
                // Attempt to extract the dependency name from the bare_key.
                let dep_name = Self::find_child_by_kind(pair_node, "bare_key")
                    .and_then(|node| node.utf8_text(self.source.as_bytes()).ok())
                    .map(|s| s.trim().to_string())?;

                // Attempt to find the inline_table and extract version info via helper.
                let version_info_opt = Self::find_child_by_kind(pair_node, "inline_table")
                    .and_then(|inline_table_node| {
                        Self::extract_version_from_inline_table(self.source, inline_table_node)
                    });

                version_info_opt.map(|(version, version_pair_text, version_str_node)| {
                    // Get the full pair text for the dependency.
                    let name_pair = pair_node
                        .utf8_text(self.source.as_bytes())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let deps_info = DepsInfo {
                        name: dep_name,
                        version,
                        name_pair,
                        version_pair: version_pair_text,
                    };
                    (version_str_node, deps_info)
                })
            })
            .collect()
    }
}

/// find_deps_in_table
impl<'a> TomlParser<'a> {
    pub fn find_deps_in_table(&self, table_node: Node<'a>) -> Option<HashMap<Node<'a>, DepsInfo>> {
        debug!("Starting to search for dependencies in table...");

        // Try to find the bare_key node that might indicate this is a dependencies table
        if let Some(bare_key_node) = Self::find_child_by_kind(table_node, "bare_key") {
            let key_text = bare_key_node
                .utf8_text(self.source.as_bytes())
                .unwrap_or("")
                .trim();

            if key_text == "dependencies" {
                let deps_info = self.extract_deps_info(table_node);

                // Only return Some if we actually found dependencies
                if !deps_info.is_empty() {
                    let mut map = HashMap::new();
                    map.extend(deps_info);
                    debug!("Returning dependencies map with {} entries", map.len());
                    return Some(map);
                }
            }
        }
        None
    }
}

/// new
impl<'a> TomlParser<'a> {
    /// Create a new TomlParser by validating and parsing the TOML source.
    /// You might want to adapt this constructor to properly initialize `pkg` and `deps`.
    pub fn new(source: &'a str) -> Result<Self, TomlParserError> {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_toml::language())
            .map_err(|_| TomlParserError::ParseError)?; // Propagate error if setting language fails.
        let tree = parser
            .parse(source, None)
            .ok_or_else(|| TomlParserError::ParseError)?;

        // Initialize with an empty HashMap for deps and pkg as None.
        Ok(Self { source, tree })
    }
}

/// find_child_by_kind
impl<'a> TomlParser<'a> {
    /// Original helper function remains available if needed.
    fn find_child_by_kind(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        node.children(&mut node.walk()).find_map(|child| {
            if child.kind() == kind {
                Some(child)
            } else {
                Self::find_child_by_kind(child, kind)
            }
        })
    }
}

/// find_package_in_table
impl<'a> TomlParser<'a> {
    pub fn find_package_in_table(&self, table_node: Node<'a>) -> Option<(Node<'a>, PkgInfo)> {
        // Check if the table has a bare_key child equal to "package"
        if let Some(bare_key_node) = Self::find_child_by_kind(table_node, "bare_key") {
            let key_text = bare_key_node
                .utf8_text(self.source.as_bytes())
                .unwrap_or("")
                .trim();
            if key_text == "package" {
                // Delegate to a helper function to search for the "name" pair
                return self.extract_pkg_info(table_node);
            }
        }
        None
    }
}

// edit_node
impl<'a> TomlParser<'a> {
    /// Edits the source code by replacing the part represented by `node` with `new_value`.
    pub fn edit_node(&self, node: Node, new_value: &str) -> String {
        // Get the positions in the source code where the node is located.
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();

        let mut new_source_code = String::new();
        new_source_code.push_str(&self.source[..start_byte]);
        new_source_code.push_str(new_value);
        new_source_code.push_str(&self.source[end_byte..]);

        new_source_code
    }
}

impl<'a> From<PackageAndDepsNodes<'a>> for PackageAndDeps {
    fn from(nodes: PackageAndDepsNodes<'a>) -> Self {
        // Debug output before conversion
        debug!("Before conversion - PackageAndDepsNodes:");
        debug!("Package: {:?}", nodes.package.as_ref().map(|(pkg, _)| pkg));
        debug!("Dependencies:");
        for (node, dep) in &nodes.dependencies {
            debug!("  Node: {:?}, Dependency: {}", node, dep);
        }

        // Perform the conversion
        let package = nodes.package.map(|(_, pkg_info)| pkg_info);
        let dependencies = nodes
            .dependencies
            .into_iter()
            .map(|(_node, deps_info)| deps_info)
            .collect();

        // Create the result
        let result = PackageAndDeps {
            package,
            dependencies,
        };

        // Debug output after conversion
        debug!("\nAfter conversion - PackageAndDeps:");
        debug!("Package: {:?}", result.package);
        debug!("Dependencies:");
        for dep in &result.dependencies {
            debug!("  {}", dep);
        }

        result
    }
}

impl fmt::Display for TomlParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TomlParserError::ParseError => write!(f, "TOML parse error"),
        }
    }
}

impl std::error::Error for TomlParserError {}

#[cfg(test)]
mod tests {
    use super::*;
    // use tree_sitter::Tree;
    use tree_sitter::Node;

    /// Helper function to locate the `[package]` table node in the syntax tree.
    fn find_package_table_node<'a>(parser: &'a TomlParser<'a>, source: &'a str) -> Option<tree_sitter::Node<'a>> {
        let root = parser.tree.root_node();
        let mut cursor = root.walk();
        root.children(&mut cursor).find(|node| {
            if node.kind() == "table" {
                if let Some(bare_key) = TomlParser::find_child_by_kind(*node, "bare_key") {
                    let key_text = bare_key.utf8_text(source.as_bytes()).unwrap_or("");
                    key_text.trim() == "package"
                } else {
                    false
                }
            } else {
                false
            }
        })
    }

    #[test]
    fn test_extract_pkg_info_success() {
        let toml_source = r#"
[package]
name = "package_test"
version = "0.4.3"
edition = "2021"
"#;
        // Create a parser instance using the new constructor.
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");

        // Find the `[package]` table node using our helper.
        let table_node = find_package_table_node(&parser, toml_source)
            .expect("The TOML should contain a [package] table");

        // Extract package information from the table.
        let pkg_info_opt = parser.extract_pkg_info(table_node);
        assert!(pkg_info_opt.is_some(), "Package info should be extracted");
        let (_version_node, pkg_info) = pkg_info_opt.unwrap();

        // Validate the extracted package fields.
        assert_eq!(pkg_info.name, "package_test", "The package name should match the expected value");
        assert_eq!(pkg_info.version, "0.4.3", "The package version should match the expected value");

        // Check that the pair strings captured contain the expected key names.
        assert!(pkg_info.name_pair.contains("name"), "The name_pair should contain 'name'");
        assert!(pkg_info.version_pair.contains("version"), "The version_pair should contain 'version'");
    }

    #[test]
    fn test_extract_pkg_info_missing_version() {
        let toml_source = r#"
[package]
name = "package_test"
edition = "2021"
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");
        let table_node = find_package_table_node(&parser, toml_source)
            .expect("The TOML should contain a [package] table");

        // Since the "version" field is missing, extraction should fail.
        let pkg_info_opt = parser.extract_pkg_info(table_node);
        assert!(pkg_info_opt.is_none(), "Package info extraction should fail if the version field is missing");
    }

    #[test]
    fn test_extract_pkg_info_missing_name() {
        let toml_source = r#"
[package]
version = "0.4.3"
edition = "2021"
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");
        let table_node = find_package_table_node(&parser, toml_source)
            .expect("The TOML should contain a [package] table");

        // Since the "name" field is missing, extraction should fail.
        let pkg_info_opt = parser.extract_pkg_info(table_node);
        assert!(pkg_info_opt.is_none(), "Package info extraction should fail if the name field is missing");
    }

    /// Test that a valid TOML with package and dependencies produces
    /// the correct PackageAndDepsNodes structure.
    #[test]
    fn test_find_package_and_deps() {
        let toml_source = r#"
[package]
name = "package_test1"
version = "0.4.3"
edition = "2021"

[dependencies]
package_test2 = { version = "0.4.3", path = "package_test2" }
        "#;

        // Create a parser instance using the new constructor.
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");
        let result = parser.find_package_and_deps();

        // Verify that we got a result.
        assert!(result.is_some(), "Package and dependencies should be extracted");

        let pkg_and_deps = result.unwrap();

        // Verify that the package info was found.
        assert!(pkg_and_deps.package.is_some(), "Package info should exist");
        let (_pkg_node, pkg_info) = pkg_and_deps.package.unwrap();
        assert_eq!(pkg_info.name, "package_test1", "Package name should match");
        assert_eq!(pkg_info.version, "0.4.3", "Package version should match");
        // We could also check name_pair and version_pair if required.
        // For example:
        // assert!(pkg_info.name_pair.contains("name ="), "Package name pair should contain key");
        // assert!(pkg_info.version_pair.contains("version ="), "Package version pair should contain key");

        // Verify that dependency info is extracted.
        // Since the dependencies are returned as a HashMap where each key is the node representing the dependency pair,
        // we can collect the DepsInfo values.
        let deps: Vec<&DepsInfo> = pkg_and_deps.dependencies.values().collect();
        assert_eq!(deps.len(), 1, "There should be exactly one dependency");

        let dep_info = deps[0];
        assert_eq!(dep_info.name, "package_test2", "Dependency name should match");
        assert_eq!(dep_info.version, "0.4.3", "Dependency version should match");
        // You can add more assertions regarding name_pair and version_pair if needed.
    }

    /// Test that TOML without a [package] table returns None.
    #[test]
    fn test_find_package_and_deps_no_package() {
        let toml_source = r#"
[dependencies]
package_test2 = { version = "0.4.3", path = "package_test2" }
        "#;

        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");
        let result = parser.find_package_and_deps();
        assert!(result.is_none(), "Should return None if package info is missing");
    }

    /// Test that a TOML file with no table nodes returns None.
    #[test]
    fn test_find_package_and_deps_no_tables() {
        let toml_source = r#"key = "value""#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");
        let result = parser.find_package_and_deps();
        assert!(result.is_none(), "Should return None if there are no table nodes");
    }

    #[test]
    fn test_extract_pkg_info_failure_when_missing_fields() {
        // In this example the `version` field is missing.
        let toml_source = r#"
[package]
name = "package_test"
edition = "2021"
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");
        let table_node = find_package_table_node(&parser, toml_source)
            .expect("The TOML should contain a [package] table");

        // Attempt to extract package information; should fail because of the missing version.
        let pkg_info_opt = parser.extract_pkg_info(table_node);
        assert!(
            pkg_info_opt.is_none(),
            "Package info extraction should fail when version is missing"
        );
    }

    #[test]
    fn test_extract_pkg_info_with_extra_fields() {
        // Test TOML with extra fields that should be ignored by the extractor.
        let toml_source = r#"
[package]
name = "another_package"
version = "1.2.3"
description = "An example package"
authors = ["Alice", "Bob"]
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");

        let table_node = find_package_table_node(&parser, toml_source)
            .expect("The TOML should contain a [package] table");

        let pkg_info_opt = parser.extract_pkg_info(table_node);
        assert!(
            pkg_info_opt.is_some(),
            "Package info should be extracted even when extra keys are present"
        );
        let (_version_node, pkg_info) = pkg_info_opt.unwrap();

        // Validate the extracted package fields for correctness.
        assert_eq!(pkg_info.name, "another_package", "The package name should match the expected value");
        assert_eq!(pkg_info.version, "1.2.3", "The package version should match the expected value");
    }

    /// Helper function to locate the `[dependencies]` table node in the syntax tree.
    fn find_dependencies_table_node<'a>(
        parser: &'a TomlParser<'a>,
        source: &'a str,
    ) -> Option<Node<'a>> {
        let root = parser.tree.root_node();
        let mut cursor = root.walk();
        root.children(&mut cursor).find(|node| {
            if node.kind() == "table" {
                if let Some(bare_key) = TomlParser::find_child_by_kind(*node, "bare_key") {
                    let key_text = bare_key.utf8_text(source.as_bytes()).unwrap_or("");
                    key_text.trim() == "dependencies"
                } else {
                    false
                }
            } else {
                false
            }
        })
    }

    #[test]
    fn test_extract_deps_info_success() {
        let toml_source = r#"
[dependencies]
package_test = { version = "0.4.3", path = "package_test" }
"#;
        // Create a parser instance using the new constructor.
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");

        // Find the `[dependencies]` table node using our helper.
        let deps_table_node = find_dependencies_table_node(&parser, toml_source)
            .expect("The TOML should contain a [dependencies] table");

        // Extract dependency information from the table.
        let deps_info = parser.extract_deps_info(deps_table_node);
        assert_eq!(
            deps_info.len(),
            1,
            "There should be exactly one dependency extracted"
        );

        // Check the extracted dependency details.
        for (_node, info) in deps_info.iter() {
            assert_eq!(
                info.name, "package_test",
                "The dependency name should be 'package_test'"
            );
            assert_eq!(
                info.version, "0.4.3",
                "The dependency version should be '0.4.3'"
            );
            assert!(
                info.name_pair.contains("package_test"),
                "The name_pair field should contain 'package_test'"
            );
            assert!(
                info.version_pair.contains("version"),
                "The version_pair field should contain 'version'"
            );
        }
    }

    #[test]
    fn test_extract_deps_info_no_inline_table() {
        // Test a dependency definition that is not using an inline table.
        let toml_source = r#"
[dependencies]
package_test = "0.4.3"
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");
        let deps_table_node = find_dependencies_table_node(&parser, toml_source)
            .expect("The TOML should contain a [dependencies] table");

        let deps_info = parser.extract_deps_info(deps_table_node);

        // Expect no dependency info extracted because the format is incorrect (missing inline table)
        assert_eq!(
            deps_info.len(),
            0,
            "No dependency should be extracted if an inline table is not present"
        );
    }

    #[test]
    fn test_extract_deps_info_missing_version() {
        // Test a dependency definition with an inline table that is missing the "version" key.
        let toml_source = r#"
[dependencies]
package_test = { path = "package_test" }
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");
        let deps_table_node = find_dependencies_table_node(&parser, toml_source)
            .expect("The TOML should contain a [dependencies] table");

        let deps_info = parser.extract_deps_info(deps_table_node);

        // Nothing should be extracted if the version is missing.
        assert_eq!(
            deps_info.len(),
            0,
            "No dependency should be extracted if the 'version' key is missing in the inline table"
        );
    }
    /// Test case where the dependency information is correctly specified.
    #[test]
    fn test_extract_deps_info_success1() {
        let toml_source = r#"
[dependencies]
package_test = { version = "0.4.3", path = "package_test" }
"#;
        // Create a parser instance using the new constructor.
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");

        // Find the `[dependencies]` table node.
        let deps_table_node = find_dependencies_table_node(&parser, toml_source)
            .expect("The TOML should contain a [dependencies] table");

        // Extract dependency information from the table.
        let deps_info = parser.extract_deps_info(deps_table_node);
        assert_eq!(
            deps_info.len(),
            1,
            "There should be exactly one dependency extracted"
        );

        // Check the extracted dependency details.
        for (_node, info) in deps_info.iter() {
            assert_eq!(
                info.name, "package_test",
                "The dependency name should be 'package_test'"
            );
            assert_eq!(
                info.version, "0.4.3",
                "The dependency version should be '0.4.3'"
            );
            assert!(
                info.name_pair.contains("package_test"),
                "The name_pair field should contain 'package_test'"
            );
            assert!(
                info.version_pair.contains("version"),
                "The version_pair field should contain 'version'"
            );
        }
    }

    /// Test case where the dependency is defined without an inline table.
    ///
    /// Here, we expect that no dependency info will be extracted since the inline table (and thus
    /// the logic to extract the version) is missing.
    #[test]
    fn test_extract_deps_info_no_inline_table1() {
        let toml_source = r#"
[dependencies]
package_test = "0.4.3"
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");

        let deps_table_node = find_dependencies_table_node(&parser, toml_source)
            .expect("The TOML should contain a [dependencies] table");

        // Since the dependency is not defined with an inline table,
        // the extraction should yield an empty map.
        let deps_info = parser.extract_deps_info(deps_table_node);
        assert!(
            deps_info.is_empty(),
            "No dependencies should be extracted when inline table is missing"
        );
    }

    /// Test case where an inline table is present but does not include a "version" key.
    ///
    /// In this case, although there is an inline table, the extraction function should not extract
    /// any dependency because the required "version" key is missing.
    #[test]
    fn test_extract_deps_info_missing_version1() {
        let toml_source = r#"
[dependencies]
package_test = { path = "package_test" }
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");

        let deps_table_node = find_dependencies_table_node(&parser, toml_source)
            .expect("The TOML should contain a [dependencies] table");

        let deps_info = parser.extract_deps_info(deps_table_node);
        assert!(
            deps_info.is_empty(),
            "No dependencies should be extracted when the 'version' key is missing"
        );
    }

    /// Test the top-level `find_deps_in_table` function.
    ///
    /// This test uses the same TOML input as the success test and verifies that the function returns
    /// Some(map) with the expected dependency.
    #[test]
    fn test_find_deps_in_table_success1() {
        let toml_source = r#"
[dependencies]
package_test = { version = "0.4.3", path = "package_test" }
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");

        // Find the `[dependencies]` table node using our helper.
        let deps_table_node = find_dependencies_table_node(&parser, toml_source)
            .expect("The TOML should contain a [dependencies] table");

        // Call the find_deps_in_table function directly.
        let deps_map_opt = parser.find_deps_in_table(deps_table_node);
        assert!(deps_map_opt.is_some(), "Expected Some(HashMap) of dependencies");
        let deps_map = deps_map_opt.unwrap();
        assert_eq!(
            deps_map.len(),
            1,
            "There should be exactly one dependency extracted in the map"
        );
    }

    /// Test the top-level `find_deps_in_table` when no dependencies table exists.
    ///
    /// When the TOML input has no `[dependencies]` table, the function should return None.
    #[test]
    fn test_find_deps_in_table_no_dependencies_table1() {
        let toml_source = r#"
[package]
name = "test_package"
version = "0.1.0"
"#;
        let parser = TomlParser::new(toml_source).expect("Parsing should succeed");

        // Attempt to find a dependencies table (expected to be None).
        let deps_table_node = find_dependencies_table_node(&parser, toml_source);
        assert!(
            deps_table_node.is_none(),
            "There should be no [dependencies] table in this TOML"
        );
    }
}
