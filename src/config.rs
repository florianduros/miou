//! Configuration file structures for the Miou bot.
//!
//! This module defines the configuration file format using TOML. The configuration
//! is split into two main sections: TMars server settings and Matrix account settings.
//!
//! # Configuration File Format
//!
//! The bot uses a TOML configuration file with the following structure:
//!
//! ```toml
//! # TMars Server Configuration
//! [tmars]
//! # Base URL of the Terraforming Mars server
//! url = "https://terraforming-mars.herokuapp.com"
//!
//! # Secret server ID for API authentication
//! server_id = "abc123xyz"
//!
//! # Polling interval in seconds (how often to check for game updates)
//! polling_interval = 120
//!
//! # Matrix Account Configuration
//! [matrix]
//! # Fully qualified Matrix user ID for the bot account
//! user_id = "@miou:matrix.org"
//!
//! # Matrix account password
//! password = "secret-password"
//!
//! # E2EE recovery passphrase
//! passphrase = "recovery-passphrase"
//! ```

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
/// Parse from a TOML file:
///
/// ```no_run
/// # use serde::Deserialize;
/// # #[derive(Deserialize)]
/// # pub struct Config {
/// #     pub tmars: TMars,
/// #     pub matrix: Matrix,
/// # }
/// # #[derive(Deserialize)]
/// # pub struct TMars {
/// #     pub url: String,
/// #     pub server_id: String,
/// #     pub polling_interval: u64,
/// # }
/// # #[derive(Deserialize)]
/// # pub struct Matrix {
/// #     pub user_id: String,
/// #     pub password: String,
/// #     pub passphrase: String,
/// # }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let toml_content = std::fs::read_to_string("config.toml")?;
/// let config: Config = toml::from_str(&toml_content)?;
///
/// println!("TMars URL: {}", config.tmars.url);
/// println!("Matrix User: {}", config.matrix.user_id);
/// # Ok(())
/// # }
/// ```
#[derive(Deserialize)]
pub struct Config {
    /// TMars server configuration
    pub tmars: TMars,
    /// Matrix account configuration
    pub matrix: Matrix,
}

/// Terraforming Mars server configuration.
///
/// Contains all settings required to connect to and poll a TMars server.
///
/// # TOML Section
///
/// ```toml
/// [tmars]
/// url = "https://terraforming-mars.herokuapp.com"
/// server_id = "your-server-id"
/// polling_interval = 120
/// ```
#[derive(Deserialize)]
pub struct TMars {
    /// Base URL of the Terraforming Mars server.
    ///
    /// Should include the protocol (http/https) but not trailing slashes.
    ///
    /// # Examples
    ///
    /// - `https://terraforming-mars.herokuapp.com`
    /// - `http://localhost:8080`
    pub url: String,

    /// Secret server ID for API authentication.
    ///
    /// This is the unique identifier for your TMars server instance,
    /// used to authenticate API requests.
    pub server_id: String,

    /// Polling interval in seconds.
    ///
    /// How frequently the bot checks the TMars server for game updates.
    pub polling_interval: u64,
}

/// Matrix account configuration.
///
/// Contains credentials and settings for the Matrix bot account.
///
/// # TOML Section
///
/// ```toml
/// [matrix]
/// user_id = "@miou:matrix.org"
/// password = "your-password"
/// passphrase = "your-recovery-passphrase"
/// ```
#[derive(Deserialize)]
pub struct Matrix {
    /// Fully qualified Matrix user ID.
    ///
    /// The Matrix ID of the bot account in the format `@username:homeserver.com`.
    ///
    /// # Examples
    ///
    /// - `@miou:matrix.org`
    /// - `@tmars-bot:example.com`
    pub user_id: String,

    /// Matrix account password.
    ///
    /// Used for initial login. After successful authentication, the session
    /// is persisted and the bot can restore without re-authenticating.
    pub password: String,

    /// E2EE recovery passphrase.
    ///
    /// Used to decrypt cross-signing keys and restore end-to-end encryption
    /// functionality. Required for participating in encrypted rooms.
    pub passphrase: String,
}
