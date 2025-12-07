//! Miou - A Matrix bot for Terraforming Mars game notifications.
//!
//! This is the main entry point for the Miou bot, which bridges Matrix messaging
//! with Terraforming Mars game servers to provide turn notifications to players.
//!
//! # Overview
//!
//! Miou is a Matrix bot that helps Terraforming Mars players stay informed about their
//! game progress. It monitors games on a TMars server and sends notifications to Matrix
//! users when it's their turn to play, after a configurable delay.
//!
//! # Features
//!
//! - **Turn Notifications**: Get notified in Matrix when it's your turn in a TMars game
//! - **Configurable Delays**: Set custom notification delays (1 minute to 1 week)
//! - **Multi-Game Support**: Monitor multiple games simultaneously
//! - **Room-Based Alerts**: Register alerts in different Matrix rooms for the same game
//! - **Automatic Cleanup**: Removes alerts when games end
//! - **Session Persistence**: Maintains Matrix login sessions across restarts
//! - **TOML Configuration**: Simple configuration file format for easy setup
//!
//! # Configuration
//!
//! Create a `config.toml` file with your settings:
//!
//! ```toml
//! [tmars]
//! url = "https://terraforming-mars.herokuapp.com"
//! server_id = "your-server-id"
//! polling_interval = 120
//!
//! [matrix]
//! user_id = "@miou:matrix.org"
//! password = "your-password"
//! passphrase = "your-recovery-passphrase"
//! ```
//!
//! # Usage
//!
//! ```bash
//! miou --config config.toml --data-path ./session
//! ```
//!
//! # Bot Commands
//!
//! Once running, users can interact with the bot using these commands in Matrix:
//!
//! - `!miou help` - Display help information
//! - `!miou games` - List all ongoing games
//! - `!miou alerts` - List your registered alerts
//! - `!miou register <game_id> <player_name> <delay>` - Register for turn notifications
//! - `!miou unregister <game_id>` - Stop receiving notifications for a game
//!
//! # Architecture
//!
//! The bot consists of several modules:
//!
//! - [`alerts`] - Alert data structures, controller, and persistence for notification management
//! - [`bot`] - Main bot logic coordinating Matrix and TMars synchronization
//! - [`commands`] - Command parsing and execution with validation
//! - [`config`] - TOML configuration file structures and loading
//! - [`matrix`] - Matrix client integration and session management
//! - [`tmars`] - TMars server API client and game state synchronization
//! - [`utils`] - Utility functions for path handling
//!
//! # Runtime Behavior
//!
//! Once started, the bot runs three concurrent tasks:
//!
//! 1. **TMars Sync Task**: Polls the TMars server every `polling_interval` seconds
//!    to fetch current game states and trigger notifications
//! 2. **Matrix Sync Task**: Listens for Matrix messages and processes bot commands
//! 3. **Alert Persistence Task**: Saves the alerts map to disk every minute
//!
//! All tasks run indefinitely until the process is terminated
//!
//! # Environment Variables
//!
//! - `RUST_LOG` - Controls logging level (default: `info`)
//!   - Set to `debug` for verbose output
//!   - Set to `warn` or `error` for minimal logging

use clap::Parser;
use env_logger::Env;
use log::error;

use crate::{bot::Bot, config::Config};

mod alerts;
mod bot;
mod commands;
mod config;
mod matrix;
mod tmars;
mod utils;

/// Command-line arguments for the Miou bot.
///
/// The bot requires two command-line arguments:
/// - A path to the TOML configuration file containing TMars and Matrix settings
/// - A path to the directory for storing persistent data (Matrix session, alerts, etc.)
///
/// Most configuration is done through the TOML file (see [`config::Config`]).
///
/// # Examples
///
/// ```bash
/// miou --config config.toml --data-path ./miou-data
/// ```
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the TOML configuration file.
    ///
    /// The configuration file should contain TMars server settings and Matrix
    /// account credentials. See the [`config`] module for the expected format.
    ///
    /// # Example
    ///
    /// ```toml
    /// [tmars]
    /// url = "https://terraforming-mars.herokuapp.com"
    /// server_id = "your-server-id"
    /// polling_interval = 120
    ///
    /// [matrix]
    /// user_id = "@miou:matrix.org"
    /// password = "your-password"
    /// passphrase = "your-recovery-passphrase"
    /// ```
    #[arg(short, long)]
    config: String,

    /// Path to the directory for storing persistent data.
    ///
    /// This directory will contain:
    /// - `session/` - Matrix session data (authentication tokens, device keys)
    /// - `alerts` - JSON file with registered alerts
    ///
    /// # Security Considerations
    ///
    /// This directory contains highly sensitive data including:
    /// - Matrix authentication tokens (allows impersonation of the bot)
    /// - End-to-end encryption keys (allows message decryption)
    /// - Cross-signing keys (critical for encryption verification)
    ///
    /// # Example
    ///
    /// ```bash
    /// # Create the data directory with restricted permissions
    /// mkdir -p ./miou-data
    /// chmod 700 ./miou-data
    ///
    /// # Run the bot
    /// miou --config config.toml --data-path ./miou-data
    /// ```
    #[arg(short, long)]
    data_path: String,
}

/// Main entry point for the Miou bot.
///
/// This function initializes the bot with the following steps:
///
/// 1. **Logging Setup**: Configures the logger with `info` level by default
///    (can be overridden with the `RUST_LOG` environment variable)
/// 2. **Argument Parsing**: Parses command-line arguments using `clap`
/// 3. **Configuration Loading**: Reads and parses the TOML configuration file
/// 4. **Bot Initialization**: Creates the bot instance, connecting to Matrix and TMars,
///    and loading any persisted alerts from disk
/// 5. **Bot Execution**: Starts the bot's main loop with three concurrent tasks:
///    - TMars sync task (polls game server)
///    - Matrix sync task (processes commands)
///    - Alert persistence task (saves alerts to disk)
///
/// # Error Handling
///
/// The function gracefully handles configuration errors by:
/// - Logging an error message if the config file cannot be read or parsed
/// - Returning early without panicking to allow for clean shutdown
///
/// Network errors during bot operation are logged but don't stop the bot.
///
/// # Panics
///
/// The function will panic if:
/// - Bot initialization fails (e.g., invalid Matrix credentials, network unreachable)
/// - The Matrix sync loop encounters an unrecoverable error
///
/// These panics indicate critical failures that prevent the bot from functioning.
///
/// # Environment Variables
///
/// - `RUST_LOG`: Controls logging verbosity (e.g., `debug`, `info`, `warn`, `error`)
///   - `debug`: Verbose output including sync details and alert processing
///   - `info`: Normal operation logs (default)
///   - `warn`: Only warnings and errors
///   - `error`: Only error messages
///
/// # Examples
///
/// Run with default log level (info):
///
/// ```bash
/// miou --config config.toml --data-path ./miou-data
/// ```
///
/// Run with debug logging to troubleshoot issues:
///
/// ```bash
/// RUST_LOG=debug miou --config config.toml --data-path ./miou-data
/// ```
///
/// Run with minimal logging:
///
/// ```bash
/// RUST_LOG=warn miou --config config.toml --data-path ./miou-data
/// ```
#[tokio::main]
async fn main() {
    // Trigger CI

    // Put logger at info level by default
    let env = Env::default().filter_or("RUST_LOG", "info");
    env_logger::init_from_env(env);

    // Parse command line arguments
    let args = Args::parse();

    // Load configuration from TOML file
    let toml_file = match std::fs::read_to_string(&args.config) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read config file: {}", e);
            return;
        }
    };

    // Parse TOML configuration
    let config: Config = match toml::from_str(&toml_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to parse config file: {}", e);
            return;
        }
    };

    let avatar_bytes = include_bytes!("../assets/miou.png");

    // Launch bot
    let bot = Bot::new(config, args, avatar_bytes).await.unwrap();
    bot.start().await;
}
