use anyhow::Context;
use anyhow::Result;
use clap::{ArgAction, Args, Parser, Subcommand};
use colored::Colorize;
use console::style;
use dialoguer::Confirm;
use log::{LevelFilter, debug};
use std::process;

mod increaser;
mod publish;

use cratup_init::{
    Config, initialize_configuration, initialize_logger, load_default_configuration,
};
use cratup_search::Search;
use increaser::Increaser;
use publish::{find_publishable_dirs, print_modules, publish_modules};

/// Configure logging verbosity using -v/--verbose and -q/--quiet flags.
#[derive(Args, Debug)]
pub struct Verbosity {
    /// Increase the level of verbosity (repeatable).
    #[arg(short = 'v', long, action = ArgAction::Count, display_order = 99)]
    pub verbose: u8,

    /// Decrease the level of verbosity (repeatable).
    #[arg(short = 'q', long, action = ArgAction::Count, display_order = 100)]
    pub quiet: u8,
}

impl Verbosity {
    pub fn log_level_filter(&self) -> LevelFilter {
        if self.quiet > 0 {
            LevelFilter::Warn
        } else {
            match self.verbose {
                0 => LevelFilter::Info,
                1 => LevelFilter::Debug,
                _ => LevelFilter::Trace,
            }
        }
    }
}

/// Mode-line interface for the module management tool.
#[derive(Parser, Debug)]
#[command(
    author = "Your Name",
    version = "0.2",
    about = "Module management tool"
)]
struct Cli {
    #[command(flatten)]
    verbose: Verbosity,

    #[command(subcommand)]
    command: Mode,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
enum Mode {
    /// Initialize configuration
    Init,

    /// Increase module version by providing the current and the next version.
    Incv(IncvArgs),

    /// Publish modules recursively found in the current directory.
    Publish,

    /// Search modules with provided criteria.
    Search(SearchArgs),
}

/// Common arguments shared by Incv and Search modes.
#[derive(Args, Debug)]
struct CommonArgs {
    /// Package name
    #[arg(short = 'p', long = "package-name", help = "Name of the package")]
    package_name: Option<String>,
}

/// Arguments for the `incv` subcommand.
#[derive(Args, Debug)]
struct IncvArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Current package version (e.g. 0.4.1)
    #[arg(
        short = 'i',
        long = "current-version",
        help = "Current version of the package (e.g. 0.4.1)"
    )]
    current_version: String,

    /// Next package version (e.g. 0.4.2)
    #[arg(
        short = 'r',
        long = "next-version",
        help = "Next version of the package (e.g. 0.4.2)"
    )]
    next_version: String,

    /// Automatically confirm the update (skip confirmation prompt)
    #[arg(
        short = 'y',
        long = "yes",
        help = "Automatically confirm the update\n"
    )]
    yes: bool,
}

/// Arguments for the `search` subcommand.
#[derive(Args, Debug)]
struct SearchArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Package version (e.g. 0.4.1)
    #[arg(
        short = 'i',
        long = "Version to search",
        help = "Version of the package (e.g. 0.4.1)"
    )]
    version: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let verbosity_level = cli.verbose.log_level_filter();
    initialize_logger(verbosity_level).context("Failed to initialize logger")?;
    debug!(
        "{} {:?}",
        style("Logger initialized with verbosity:").cyan(),
        verbosity_level
    );

    let config = load_default_configuration().context("Failed to load default configuration")?;
    debug!("{}", style("Default configuration loaded").green());

    match &cli.command {
        Mode::Init => {
            debug!("{}", style("Initializing configuration...").yellow());
            initialize_configuration().context("Failed to initialize configuration")?;
        }
        Mode::Incv(args) => {
            if let Some(ref package) = args.common.package_name {
                debug!(
                    "Running incv mode for package {}: updating version from {} to {}",
                    package, args.current_version, args.next_version
                );
            } else {
                debug!(
                    "Running incv mode: updating version from {} to {}",
                    args.current_version, args.next_version
                );
            }
            // Pass the config as a parameter to run_incv.
            if let Err(e) = run_incv(args, &config) {
                eprintln!("Error updating version: {}", e);
                std::process::exit(1);
            }
        }
        Mode::Publish => {
            debug!("Running publish mode: publishing modules recursively");
            if let Err(e) = run_publish() {
                eprintln!("Error publishing modules: {}", e);
                std::process::exit(1);
            }
        }
        Mode::Search(args) => {
            if let Some(ref package) = args.common.package_name {
                debug!(
                    "Running search mode for package {} with current version {:?}",
                    package, args.version
                );
            } else {
                debug!(
                    "Running search mode with current version {:?}",
                    args.version
                );
            }
            if let Err(e) = run_search(args) {
                eprintln!("Error during search: {}", e);
                std::process::exit(1);
            }
        }
    }

    debug!("Execution completed successfully");
    Ok(())
}

/// The run function for the increaser. It extracts parameters from the command-line options,
/// retrieves the current directory, and then creates an Increaser instance to perform the update.
fn run_incv(args: &IncvArgs, config: &Config) -> Result<()> {
    debug!("Starting version increment process with args: {:?}", args);

    // Retrieve the current working directory as a string.
    let current_dir = std::env::current_dir().with_context(|| {
        debug!("Failed to get current working directory");
        "Failed to get current directory"
    })?;
    debug!("Current working directory: {:?}", current_dir);

    // Initialize the increaser.
    debug!(
        "Creating Increaser with current_version: {}, next_version: {}, package_name: {:?}",
        args.current_version, args.next_version, args.common.package_name
    );
    let increaser = Increaser::new(
        current_dir,
        args.current_version.clone(),
        args.next_version.clone(),
        args.common.package_name.clone(),
    )
    .with_context(|| {
        debug!("Failed to initialize Increaser");
        "Failed to initialize version increaser"
    })?;
    debug!("Increaser initialized successfully");

    // Print current version matches.
    debug!("Printing current version matches");
    increaser.print_current_version_matches().with_context(|| {
        debug!("Failed while printing current version matches");
        "Failed to print current version matches"
    })?;

    // Decide if we need to ask for confirmation.
    if args.yes {
        debug!("CLI flag 'yes' provided: skipping confirmation");
    } else if config.always_ask_permission {
        // Only ask if the configuration indicates it.
        ask_to_continue();
        debug!("User confirmed continuation via config-based prompt");
    } else {
        debug!("No confirmation required");
    }

    // Execute the update process.
    debug!("Starting directory and package updates");
    let _updated_packages = increaser.update_dirs_and_packages().with_context(|| {
        debug!("Failed during directory and package updates");
        "Failed to update directories and packages"
    })?;
    debug!("Successfully updated directories and packages");

    println!("Updated packages:");
    // Print next version matches.
    debug!("Printing next version matches");
    increaser.print_next_version_matches().with_context(|| {
        debug!("Failed while printing next version matches");
        "Failed to print next version matches"
    })?;

    debug!("Version increment process completed successfully");
    Ok(())
}

fn run_search(args: &SearchArgs) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Starting search operation with args: {:?}", args);

    // Retrieve the current working directory.
    let current_dir = std::env::current_dir().map_err(|e| {
        debug!("Failed to get current directory: {}", e);
        "Failed to get current directory"
    })?;
    debug!("Current working directory: {:?}", current_dir);

    // Create a new Search instance. Note: the constructor only loads raw data.
    debug!(
        "Initializing Search with version: {:?}, package_name: {:?}",
        args.version, args.common.package_name
    );
    let mut search_instance = Search::new(
        current_dir,
        args.version.clone(),
        args.common.package_name.clone(),
    )
    .map_err(|e| {
        debug!("Search initialization failed: {:?}", e);
        "Failed to initialize search"
    })?;
    debug!("Search instance created successfully");

    // Run the normal search using filtering functions.
    search_instance.search()?;
    // Retrieve the found packages from the updated field.
    let mut found_packages = search_instance.pkg_deps_dirs.clone();
    debug!("Search returned {} result(s)", found_packages.len());

    // If the search returns no results, try fuzzy search.
    if found_packages.is_empty() {
        debug!("No results found in search; executing fuzzy search for the closest match");
        found_packages = search_instance.fuzzy_search()?;
        if !found_packages.is_empty() {
            // Print the fuzzy found package information on screen.
            for (path, pkg_and_deps) in &found_packages {
                println!("Found similar package (exact package name not found): {}", pkg_and_deps.package.clone().unwrap().name.green());
                debug!(
                    "Fuzzy search found package at {:?}: {:?}",
                    path, pkg_and_deps
                );
            }
        } else {
            println!("No packages found, even with fuzzy search.");
        }
    } else {
        // Display the found packages with blue version coloring.
        debug!("Executing search display with blue version coloring");
        search_instance.display(|s| {
            debug!("Applying blue color to version string: {}", s);
            s.green()
        });
    }
    debug!("Search operation completed successfully");
    Ok(())
}

fn run_publish() -> Result<()> {
    // Get the current directory.
    let current_dir = std::env::current_dir()?;
    debug!("Current directory: {:?}", current_dir);

    // Find publishable directories.
    let publishable_dirs = find_publishable_dirs(&current_dir);
    debug!(
        "Total publishable directories found: {}",
        publishable_dirs.len()
    );

    // Publish each module and obtain the final publish states.
    let publish_states = publish_modules(&publishable_dirs)?;

    // Print the published modules in green and unpublished in red.
    print_modules(&publish_states);

    Ok(())
}

fn ask_to_continue() {
    // Prompt the user with a yes/no question. If the user presses enter, the default value (false) is returned.
    let continue_execution = Confirm::new()
        .with_prompt("Proceed to bump all to the new version? (No/yes)")
        .default(false)  // default is "No" if enter is pressed
        .interact()
        .unwrap();

    if continue_execution {
    } else {
        println!("Execution interrupted.");
        process::exit(1);
    }
}
