//! Bot command parsing and response formatting.
//!
//! This module provides the complete command processing pipeline for the Miou bot,
//! enabling Matrix users to interact with Terraforming Mars game notifications.
//!
//! # Overview
//!
//! The commands module handles the entire lifecycle of bot commands:
//! 1. **Parsing** - Converting Matrix messages into structured [`command::Command`] enums
//! 2. **Validation** - Ensuring commands have correct syntax and valid arguments
//! 3. **Execution** - Routing commands to specialized handlers
//! 4. **Response** - Formatting results as Markdown for Matrix display
//! 5. **State Management** - Managing alert subscriptions
//!
//! # Architecture
//!
//! ```text
//! Matrix Message
//!      │
//!      ▼
//! ┌─────────────┐
//! │  Commander  │  ← Entry point: parse() + parse_command()
//! └─────────────┘
//!      │
//!      ├── parse() ────────────────────┐
//!      │                               ▼
//!                          ┌──────────────────┐
//!                          │  command::Command│
//!                          └──────────────────┘
//!      │
//!      └── parse_command() ───────────┐
//!                                     ▼
//!                          ┌─────────────────────┐
//!                          │ Action Handlers     │
//!                          │  - handle_help      │
//!                          │  - handle_games     │
//!                          │  - handle_alerts    │
//!                          │  - handle_register  │
//!                          │  - handle_unregister│
//!                          └─────────────────────┘
//!                                     │
//!                                     ▼
//!                          ┌────────────────────┐
//!                          │  CommandResult     │
//!                          │  - response (MD)   │
//!                          │  - alert changes   │
//!                          └────────────────────┘
//! ```
//!
//! # Command Structure
//!
//! All commands follow the format: `!miou <subcommand> [args...]`
//!
//! ## Available Commands
//!
//! | Command | Arguments | Description |
//! |---------|-----------|-------------|
//! | `help` | None | Display help information |
//! | `games` | None | List all ongoing Terraforming Mars games |
//! | `alerts` | None | List active alert registrations |
//! | `register` | `<game_id> <player_name> <delay>` | Register for turn notifications |
//! | `unregister` | `<game_id>` | Stop receiving notifications for a game |
//!
//! ## Command Details
//!
//! ### Register Command
//!
//! Subscribes to notifications when a player's turn arrives in a game.
//!
//! - **game_id**: The unique identifier of the Terraforming Mars game
//! - **player_name**: The name of the player to monitor
//! - **delay**: Minutes to wait before notifying (1-10080, i.e., 1 week max)
//!
//! ### Unregister Command
//!
//! Removes all alert subscriptions for a specific game in the current room.
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! # use miou::commands::Commander;
//! # use std::collections::HashMap;
//! # async fn example() {
//! use miou::commands::{Commander, CommandContext};
//!
//! let commander = Commander::new();
//!
//! // Parse a Matrix message
//! let message = "!miou games".to_string();
//! match commander.parse(message) {
//!     Ok(command) => {
//!         // Create execution context
//!         let context = CommandContext {
//!             games_map: HashMap::new(),
//!             alerts_map: HashMap::new(),
//!             room_id: "!room:example.com".to_string(),
//!             user_id: "@user:example.com".to_string(),
//!         };
//!
//!         // Execute the command
//!         if let Some(result) = commander.parse_command(&command, &context).await {
//!             println!("Bot response: {}", result.response);
//!         }
//!     }
//!     Err(e) => {
//!         // Handle parse errors (invalid commands, wrong bot, etc.)
//!     }
//! }
//! # }
//! ```
//!
//! ## Command Parsing
//!
//! ```
//! # use miou::commands::Commander;
//! # use miou::commands::command::Command;
//!
//! let commander = Commander::new();
//!
//! // Help command
//! let cmd = commander.parse("!miou help".to_string()).unwrap();
//! assert!(matches!(cmd, Command::Help));
//!
//! // Register command
//! let cmd = commander.parse("!miou register game123 Alice 60".to_string()).unwrap();
//! if let Command::Register(game_id, player_name, delay) = cmd {
//!     assert_eq!(game_id, "game123");
//!     assert_eq!(player_name, "Alice");
//!     assert_eq!(delay, 60);
//! }
//! ```
//!
//! # Error Handling
//!
//! The module distinguishes between two error categories:
//!
//! - **Silent Errors** ([`CommandParseError::NotForBot`]): Messages that aren't commands
//!   or are for a different bot. These should not generate responses.
//!
//! - **User Errors** ([`CommandParseError::InvalidCommand`]): Invalid command syntax
//!   or arguments. These include helpful error messages for the user.
//!
//! # Module Organization
//!
//! - [`commander`] - Main orchestrator for parsing and executing commands
//! - [`command`] - Command enum definitions and parsing logic
//! - [`actions`] - Individual command handler implementations
//! - [`markdown_response`] - Response formatting utilities

use std::collections::{HashMap, HashSet};

mod actions;
mod command;
mod commander;
mod markdown_response;

pub use crate::commands::commander::Commander;
use crate::{alerts::Alert, tmars::Game};

/// Runtime context for command execution.
///
/// This structure provides all the state and metadata needed to execute
/// a command. It's passed to command handlers during the execution phase.
///
/// # Fields
///
/// * `games_map` - All active Terraforming Mars games, indexed by game ID
/// * `alerts_map` - Alert subscriptions, indexed by game ID with sets of [`Alert`]s
/// * `room_id` - Matrix room ID where the command was issued
/// * `user_id` - Matrix user ID of the user who issued the command
///
/// # Examples
///
/// ```
/// # use miou::commands::CommandContext;
/// # use std::collections::HashMap;
/// let context = CommandContext {
///     games_map: HashMap::new(),
///     alerts_map: HashMap::new(),
///     room_id: "!room:example.com".to_string(),
///     user_id: "@user:example.com".to_string(),
/// };
/// ```
#[derive(Debug)]
pub struct CommandContext {
    /// Map of active games indexed by game ID
    pub games_map: HashMap<String, Game>,
    /// Map of alert subscriptions indexed by game ID
    pub alerts_map: HashMap<String, HashSet<Alert>>,
    /// Matrix room ID where the command was issued
    pub room_id: String,
    /// Matrix user ID of the command issuer
    pub user_id: String,
}

/// Result of command execution.
///
/// This structure encapsulates the outcome of a command handler, including
/// the response to send to the user and any state changes to apply.
///
/// # Fields
///
/// * `response` - Markdown-formatted message to send to the Matrix room
/// * `alert_to_add` - Optional alert to register: (game_id, Alert)
/// * `alerts_to_remove` - Optional alerts to remove: (game_id, room_id, user_id)
///
/// # State Changes
///
/// Command handlers don't directly modify state. Instead, they return state
/// change requests through `alert_to_add` and `alerts_to_remove`. The caller
/// is responsible for applying these changes.
///
/// # Examples
///
/// ```
/// # use miou::commands::CommandResult;
/// // Read-only command (help, games, alerts)
/// let result = CommandResult {
///     response: "No ongoing games found.".to_string(),
///     alert_to_add: None,
///     alerts_to_remove: None,
/// };
/// ```
#[derive(Debug)]
pub struct CommandResult {
    /// Markdown-formatted response message
    pub response: String,
    /// Optional alert to register: (game_id, Alert)
    pub alert_to_add: Option<(String, Alert)>,
    /// Optional alerts to remove: (game_id, room_id, user_id)
    pub alerts_to_remove: Option<(String, String, String)>,
}

/// Errors that can occur during command parsing.
///
/// This enum distinguishes between errors that should produce user-facing
/// messages and those that should be silently ignored.
///
/// # Variants
///
/// * `NotForBot` - Message is not a command or is for a different bot.
///   Should be handled silently without responding to the user.
///
/// * `InvalidCommand` - Command syntax or arguments are invalid.
///   Contains a user-friendly error message to display.
///
/// # Examples
///
/// ```
/// # use miou::commands::{Commander, CommandParseError};
/// let commander = Commander::new();
///
/// // Not a command - silent error
/// match commander.parse("Just chatting".to_string()) {
///     Err(CommandParseError::NotForBot) => {
///         // Don't respond - not a command
///     }
///     _ => {}
/// }
///
/// // Invalid command - send error message
/// match commander.parse("!miou invalid_cmd".to_string()) {
///     Err(CommandParseError::InvalidCommand(msg)) => {
///         // Send error message to user
///         println!("Error: {}", msg);
///     }
///     _ => {}
/// }
/// ```
#[derive(Debug)]
pub enum CommandParseError {
    /// Message is not for this bot (silent error)
    NotForBot,
    /// Invalid command syntax with error message
    InvalidCommand(String),
}
