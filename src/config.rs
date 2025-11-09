//! Configuration file structures for the Miou bot.
//!
//! This module defines the configuration file format using YAML. The configuration
//! is split into two main sections: TMars server settings and Matrix account settings.
//!
//! # Configuration File Format
//!
//! The bot uses a YAML configuration file with the following structure:
//!
//! ```yaml
//! # TMars Server Configuration
//! tmars:
//!   # Base URL of the Terraforming Mars server
//!   url: "https://terraforming-mars.herokuapp.com"
//!
//!   # Secret server ID for API authentication
//!   server_id: "abc123xyz"
//!
//!   # Polling interval in seconds (how often to check for game updates)
//!   polling_interval: 120
//!
//! # Matrix Account Configuration
//! matrix:
//!   # Fully qualified Matrix user ID for the bot account
//!   # IMPORTANT: Quote the value because @ is a special character in YAML
//!   user_id: "@miou:matrix.org"
//!
//!   # Matrix account password
//!   password: "secret-password"
//!
//!   # E2EE recovery passphrase
//!   passphrase: "recovery-passphrase"
//! ```
//!
//! # Environment Variables
//!
//! Environment variables can override configuration values using the `MIOU_` prefix.
//! The structure follows the nested path in the YAML file, separated by double underscores (`__`).
//!
//! Examples:
//! - `MIOU_TMARS__URL` overrides `tmars.url`
//! - `MIOU_TMARS__SERVER_ID` overrides `tmars.server_id`
//! - `MIOU_MATRIX__USER_ID` overrides `matrix.user_id`
//! - `MIOU_MATRIX__PASSWORD` overrides `matrix.password`
//!
//! ```bash
//! export MIOU_TMARS__URL="https://terraforming-mars.herokuapp.com"
//! export MIOU_TMARS__SERVER_ID="your-server-id"
//! export MIOU_MATRIX__USER_ID="@miou:matrix.org"
//! export MIOU_MATRIX__PASSWORD="your-password"
//! export MIOU_MATRIX__PASSPHRASE="your-passphrase"
//! miou --config config.yaml --data-path ./data
//! ```

use figment::{
    Figment,
    providers::{Env, Format, Yaml},
};
use serde::Deserialize;

/// Root configuration structure for the Miou bot.
///
/// This structure represents the complete bot configuration, containing both
/// TMars server settings and Matrix account credentials.
///
/// # Structure
///
/// The configuration is divided into two sections:
/// - [`TMars`] - Terraforming Mars server connection settings
/// - [`Matrix`] - Matrix account credentials and settings
///
/// # Examples
///
/// Parse from a YAML file:
///
/// ```no_run
/// # use miou::config::Config;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = Config::load("config.yaml")?;
///
/// println!("TMars URL: {}", config.tmars.url);
/// println!("Matrix User: {}", config.matrix.user_id);
/// # Ok(())
/// # }
/// ```
#[derive(Deserialize, Debug)]
pub struct Config {
    /// TMars server configuration
    pub tmars: TMars,
    /// Matrix account configuration
    pub matrix: Matrix,
}

impl Config {
    /// Load configuration from a YAML file with environment variable overrides.
    ///
    /// This function reads the YAML file and merges it with environment variables
    /// that use the `MIOU_` prefix. Environment variables take precedence over
    /// file values.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the YAML configuration file
    ///
    /// # Returns
    ///
    /// Returns the parsed configuration or an error if:
    /// - The file cannot be read
    /// - The YAML is invalid
    /// - Required fields are missing
    /// - Types are incorrect
    ///
    /// # Environment Variable Format
    ///
    /// Environment variables use the `MIOU_` prefix followed by the configuration
    /// path with sections separated by double underscores (`__`):
    ///
    /// - `MIOU_TMARS__URL` → `tmars.url`
    /// - `MIOU_TMARS__SERVER_ID` → `tmars.server_id`
    /// - `MIOU_TMARS__POLLING_INTERVAL` → `tmars.polling_interval`
    /// - `MIOU_MATRIX__USER_ID` → `matrix.user_id`
    /// - `MIOU_MATRIX__PASSWORD` → `matrix.password`
    /// - `MIOU_MATRIX__PASSPHRASE` → `matrix.passphrase`
    ///
    /// # Examples
    ///
    /// Load from file only:
    /// ```no_run
    /// # use miou::config::Config;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::load("config.yaml")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Load from file with environment variable overrides:
    /// ```no_run
    /// # use miou::config::Config;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// std::env::set_var("MIOU_MATRIX__PASSWORD", "secret-from-env");
    /// let config = Config::load("config.yaml")?;
    /// // config.matrix.password will be "secret-from-env"
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::result_large_err)]
    pub fn load(path: &str) -> Result<Self, figment::Error> {
        Figment::new()
            .merge(Yaml::file(path))
            .merge(Env::prefixed("MIOU_").split("__"))
            .extract()
    }
}

/// Terraforming Mars server configuration.
///
/// Contains all settings required to connect to and poll a TMars server.
///
/// # YAML Section
///
/// ```yaml
/// tmars:
///   url: "https://terraforming-mars.herokuapp.com"
///   server_id: "your-server-id"
///   polling_interval: 120
/// ```
///
/// # Environment Variables
///
/// - `MIOU_TMARS__URL`
/// - `MIOU_TMARS__SERVER_ID`
/// - `MIOU_TMARS__POLLING_INTERVAL`
#[derive(Deserialize, Debug)]
pub struct TMars {
    /// Base URL of the Terraforming Mars server.
    ///
    /// # Examples
    ///
    /// - `https://terraforming-mars.herokuapp.com`
    /// - `http://localhost:8080`
    ///
    /// # Environment Variable
    ///
    /// Can be overridden with `MIOU_TMARS__URL`
    pub url: String,

    /// Secret server ID for API authentication.
    ///
    /// This is the unique identifier for your TMars server instance,
    /// used to authenticate API requests.
    ///
    /// # Environment Variable
    ///
    /// Can be overridden with `MIOU_TMARS__SERVER_ID`
    pub server_id: String,

    /// Polling interval in seconds.
    ///
    /// How frequently the bot checks the TMars server for game updates.
    ///
    /// # Environment Variable
    ///
    /// Can be overridden with `MIOU_TMARS__POLLING_INTERVAL`
    pub polling_interval: u64,
}

/// Matrix account configuration.
///
/// Contains credentials and settings for the Matrix bot account.
///
/// # YAML Section
///
/// ```yaml
/// matrix:
///   user_id: "@miou:matrix.org"
///   password: "your-password"
///   passphrase: "your-recovery-passphrase"
/// ```
///
/// # Environment Variables
///
/// - `MIOU_MATRIX__USER_ID`
/// - `MIOU_MATRIX__PASSWORD`
/// - `MIOU_MATRIX__PASSPHRASE`
#[derive(Deserialize, Debug)]
pub struct Matrix {
    /// Fully qualified Matrix user ID.
    ///
    /// The Matrix ID of the bot account in the format `@username:homeserver.com`.
    ///
    /// # Examples
    ///
    /// - `@miou:matrix.org`
    /// - `@tmars-bot:example.com`
    ///
    /// # Environment Variable
    ///
    /// Can be overridden with `MIOU_MATRIX__USER_ID`
    pub user_id: String,

    /// Matrix account password.
    ///
    /// Used for initial login. After successful authentication, the session
    /// is persisted and the bot can restore without re-authenticating.
    ///
    /// # Environment Variable
    ///
    /// Can be overridden with `MIOU_MATRIX__PASSWORD`
    pub password: String,

    /// E2EE recovery passphrase.
    ///
    /// Used to decrypt cross-signing keys and restore end-to-end encryption
    /// functionality. Required for participating in encrypted rooms.
    ///
    /// # Environment Variable
    ///
    /// Can be overridden with `MIOU_MATRIX__PASSPHRASE`
    pub passphrase: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    // Helper to clean up all MIOU_ env vars before each test
    fn cleanup_env_vars() {
        let vars_to_clean = [
            "MIOU_TMARS__URL",
            "MIOU_TMARS__SERVER_ID",
            "MIOU_TMARS__POLLING_INTERVAL",
            "MIOU_MATRIX__USER_ID",
            "MIOU_MATRIX__PASSWORD",
            "MIOU_MATRIX__PASSPHRASE",
        ];

        unsafe {
            for var in &vars_to_clean {
                env::remove_var(var);
            }
        }
    }

    #[test]
    #[serial]
    fn test_load_from_yaml_file() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
tmars:
  url: "http://localhost:9090"
  server_id: "test123"
  polling_interval: 60

matrix:
  user_id: "@bot:matrix.org"
  password: "pass123"
  passphrase: "phrase123"
"#;

        fs::write(&config_path, yaml_content).unwrap();

        let config = Config::load(config_path.to_str().unwrap()).unwrap();

        assert_eq!(config.tmars.url, "http://localhost:9090");
        assert_eq!(config.tmars.server_id, "test123");
        assert_eq!(config.tmars.polling_interval, 60);
        assert_eq!(config.matrix.user_id, "@bot:matrix.org");
        assert_eq!(config.matrix.password, "pass123");
        assert_eq!(config.matrix.passphrase, "phrase123");

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_env_var_overrides() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
tmars:
  url: "http://localhost:9090"
  server_id: "test123"
  polling_interval: 60

matrix:
  user_id: "@bot:matrix.org"
  password: "pass123"
  passphrase: "phrase123"
"#;

        fs::write(&config_path, yaml_content).unwrap();

        // Set environment variables
        unsafe {
            env::set_var("MIOU_TMARS__URL", "http://env-override:8080");
            env::set_var("MIOU_MATRIX__PASSWORD", "env-password");
        }

        let config = Config::load(config_path.to_str().unwrap()).unwrap();

        // Check that env vars override file values
        assert_eq!(config.tmars.url, "http://env-override:8080");
        assert_eq!(config.matrix.password, "env-password");

        // Check that non-overridden values remain from file
        assert_eq!(config.tmars.server_id, "test123");
        assert_eq!(config.matrix.user_id, "@bot:matrix.org");

        // Cleanup
        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_missing_file_error() {
        cleanup_env_vars();

        let result = Config::load("/nonexistent/path/that/does/not/exist/config.yaml");
        assert!(result.is_err());

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_invalid_yaml_error() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let invalid_yaml = r#"
tmars:
  url: [this is not valid yaml
"#;

        fs::write(&config_path, invalid_yaml).unwrap();

        let result = Config::load(config_path.to_str().unwrap());
        assert!(result.is_err());

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_missing_required_field() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let incomplete_yaml = r#"
tmars:
  url: "http://localhost:9090"
  server_id: "test123"
  # Missing polling_interval
"#;

        fs::write(&config_path, incomplete_yaml).unwrap();

        let result = Config::load(config_path.to_str().unwrap());
        assert!(result.is_err());

        cleanup_env_vars();
    }

    #[test]
    #[serial]
    fn test_all_from_env_vars() {
        cleanup_env_vars();

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Create a minimal YAML file
        let yaml_content = r#"
tmars:
  url: "placeholder"
  server_id: "placeholder"
  polling_interval: 1

matrix:
  user_id: "placeholder"
  password: "placeholder"
  passphrase: "placeholder"
"#;

        fs::write(&config_path, yaml_content).unwrap();

        // Override everything with env vars
        unsafe {
            env::set_var("MIOU_TMARS__URL", "http://env-only:9090");
            env::set_var("MIOU_TMARS__SERVER_ID", "env-server");
            env::set_var("MIOU_TMARS__POLLING_INTERVAL", "120");
            env::set_var("MIOU_MATRIX__USER_ID", "@env:matrix.org");
            env::set_var("MIOU_MATRIX__PASSWORD", "env-pass");
            env::set_var("MIOU_MATRIX__PASSPHRASE", "env-phrase");
        }

        let config = Config::load(config_path.to_str().unwrap()).unwrap();

        assert_eq!(config.tmars.url, "http://env-only:9090");
        assert_eq!(config.tmars.server_id, "env-server");
        assert_eq!(config.tmars.polling_interval, 120);
        assert_eq!(config.matrix.user_id, "@env:matrix.org");
        assert_eq!(config.matrix.password, "env-pass");
        assert_eq!(config.matrix.passphrase, "env-phrase");

        // Cleanup
        cleanup_env_vars();
    }
}
