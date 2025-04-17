use anyhow::{Context, Result};
use dialoguer::Input;
use log::{debug, warn};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// Always ask for permission to modify files.
    pub always_ask_permission: bool,
}

// Manually implement Default to set the custom default value.
impl Default for Config {
    fn default() -> Self {
        Config {
            always_ask_permission: false, // Default is No.
        }
    }
}

/// Initializes and updates the configuration for file modification permission.
///
/// This function loads the existing configuration, prompts the user with a yes/no question,
/// and saves the updated boolean value.
pub fn initialize_configuration() -> Result<()> {
    debug!("Initializing configuration process started.");

    // Load the configuration using confy.
    let mut config: Config = confy::load("cratup_auto", "config")
        .context("Failed to load configuration")?;

    // Determine the current setting as a string.
    let current_value = if config.always_ask_permission { "yes" } else { "no" };

    // Prompt the user for permission to modify files.
    let input: String = Input::new()
        .with_prompt(format!(
            "Always ask for permission to modify files? (yes/no, current: {})",
            current_value
        ))
        .default(String::from(current_value))
        .interact_text()?;

    // Parse the input: only exactly "yes" (case-sensitive) counts as true.
    config.always_ask_permission = if input == "yes" { true } else { false };

    debug!(
        "User input received for always_ask_permission: {}",
        config.always_ask_permission
    );

    // Save the updated configuration.
    confy::store("cratup_auto", "config", &config)
        .context("Failed to save configuration")?;
    debug!("Configuration saved successfully.");

    Ok(())
}

/// Loads and provides default configuration settings for the application.
///
/// This function attempts to load existing configuration settings and falls
/// back to default values if none are found.
///
/// # Parameters
///
/// # Returns
/// - `Result<Config>`: The loaded or default configuration settings.
///
/// # Notes
/// - If configuration loading fails, default values will be used.
pub fn load_default_configuration() -> Result<Config> {
    debug!("Default configuration loading using confy...");

    // Attempt to load the configuration using confy
    match confy::load("cratup_auto", "config") {
        Ok(config) => {
            debug!("Configuration successfully loaded.");
            Ok(config)
        }
        Err(err) => {
            warn!(
                "Failed to load configuration: {}. Using default configuration.",
                err
            );
            Ok(Config::default())
        }
    }
}
