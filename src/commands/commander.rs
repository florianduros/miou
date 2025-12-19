//! Command orchestration and execution.
//!
//! This module provides the [`Commander`] struct, which serves as the main entry point
//! for processing bot commands. It coordinates command parsing and execution, routing
//! commands to their appropriate handlers.
//!
//! # Architecture
//!
//! The Commander follows a two-phase processing model:
//!
//! 1. **Parsing Phase** - Validates and parses raw message text into structured [`Command`] enums
//! 2. **Execution Phase** - Routes parsed commands to specialized handlers that produce results
//!
//! # Flow
//!
//! ```text
//! Matrix Message → parse() → Command → parse_command() → CommandResult
//! ```
//!
//! # Examples
//!
//! ```no_run
//! # use miou::commands::{Commander, CommandContext};
//! # use std::collections::HashMap;
//! # async fn example() {
//! let commander = Commander::new();
//!
//! // Parse a message
//! let message = "!miou help".to_string();
//! let command = commander.parse(message).unwrap();
//!
//! // Execute the command
//! let context = CommandContext {
//!     games_map: HashMap::new(),
//!     alerts_map: HashMap::new(),
//!     room_id: "!room:example.com".to_string(),
//!     user_id: "@user:example.com".to_string(),
//! };
//! let result = commander.parse_command(&command, &context).await;
//! # }
//! ```

use command_parser::Parser;

use crate::commands::{
    CommandContext, CommandParseError, CommandResult,
    actions::{handle_alerts, handle_games, handle_help, handle_register, handle_unregister},
    command::{Command, format_command_error},
    markdown_response::{format_access_error, format_player_turn},
};

/// Command orchestrator for parsing and executing bot commands.
///
/// The Commander is responsible for:
/// - Parsing raw message text into structured commands
/// - Validating command syntax and arguments
/// - Routing commands to appropriate handlers
/// - Converting errors into user-friendly messages
///
/// # Command Prefix
///
/// All commands must start with the `!miou` prefix. Messages without this prefix
/// are silently ignored (returning [`CommandParseError::NotForBot`]).
///
/// # Supported Commands
///
/// - `help` - Display help information
/// - `games` - List all ongoing games
/// - `alerts` - List user's registered alerts
/// - `register <game_id> <player_name> <delay>` - Register for turn notifications
/// - `unregister <game_id>` - Stop receiving notifications
pub struct Commander {
    /// Command parser for processing user commands
    parser: Parser,
}

impl Commander {
    /// Creates a new Commander instance with a configured command parser.
    ///
    /// The parser is configured to recognize commands starting with `!` as the command
    /// prefix and `-` as the option prefix.
    ///
    /// # Returns
    ///
    /// A new `Commander` instance ready to parse commands.
    ///
    /// # Examples
    ///
    /// ```
    /// # use miou::commands::Commander;
    /// let commander = Commander::new();
    /// ```
    pub fn new() -> Self {
        let parser = Parser::new('!', '-');
        Commander { parser }
    }

    /// Parses a Matrix message body into a structured command.
    ///
    /// This method validates that the message is:
    /// 1. A valid command format (starts with `!`)
    /// 2. Directed at this bot (uses `miou` as the command name)
    /// 3. Contains valid syntax and arguments
    ///
    /// # Arguments
    ///
    /// * `body` - The raw message text from Matrix
    ///
    /// # Returns
    ///
    /// * `Ok(Command)` - Successfully parsed and validated command
    /// * `Err(CommandParseError::NotForBot)` - Message is not a command or for a different bot
    /// * `Err(CommandParseError::InvalidCommand)` - Command syntax is invalid
    ///
    /// # Error Handling
    ///
    /// - Non-command messages return `NotForBot` to avoid responding to regular chat
    /// - Invalid command syntax returns `InvalidCommand` with a user-friendly error message
    /// - Commands for other bots (e.g., `!other_bot`) return `NotForBot`
    ///
    /// # Examples
    ///
    /// ```
    /// # use miou::commands::Commander;
    /// let commander = Commander::new();
    ///
    /// // Valid command
    /// let result = commander.parse("!miou help".to_string());
    /// assert!(result.is_ok());
    ///
    /// // Not a command
    /// let result = commander.parse("Hello, world!".to_string());
    /// assert!(result.is_err());
    ///
    /// // Wrong bot
    /// let result = commander.parse("!other_bot help".to_string());
    /// assert!(result.is_err());
    /// ```
    pub fn parse(&self, body: &str) -> Result<Command, CommandParseError> {
        let parse_result = Command::parse(&self.parser, body);

        // Raise an error message if the command is invalid
        if parse_result.is_err() {
            let error = parse_result.err().unwrap();
            // Return silently if the command is not for the bot
            // Otherwise, send an error message
            if let Some(message) = format_command_error(error) {
                return Err(CommandParseError::InvalidCommand(message));
            }
            return Err(CommandParseError::NotForBot);
        }

        Ok(parse_result.unwrap())
    }

    /// Executes a parsed command and returns the result.
    ///
    /// This method routes commands to their appropriate handlers and collects the
    /// results.
    ///
    /// # Arguments
    ///
    /// * `command` - The parsed command to execute
    /// * `context` - Runtime context containing:
    ///   - `games_map` - Current active games
    ///   - `alerts_map` - User alert registrations
    ///   - `room_id` - Matrix room where command was issued
    ///   - `user_id` - Matrix user who issued the command
    ///
    /// # Returns
    ///
    /// * `Some(CommandResult)` - Command executed successfully with a result
    /// * `None` - Command handler rejected the command (invalid command type)
    ///
    /// # Command Handlers
    ///
    /// - [`Command::Help`] → [`handle_help`]
    /// - [`Command::Games`] → [`handle_games`]
    /// - [`Command::Alerts`] → [`handle_alerts`]
    /// - [`Command::Register`] → [`handle_register`]
    /// - [`Command::Unregister`] → [`handle_unregister`]
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use miou::commands::{Commander, Command, CommandContext};
    /// # use std::collections::HashMap;
    /// # async fn example() {
    /// let commander = Commander::new();
    /// let command = Command::Help;
    /// let context = CommandContext {
    ///     games_map: HashMap::new(),
    ///     alerts_map: HashMap::new(),
    ///     room_id: "!room:example.com".to_string(),
    ///     user_id: "@user:example.com".to_string(),
    /// };
    ///
    /// if let Some(result) = commander.parse_command(&command, &context).await {
    ///     println!("Response: {}", result.response);
    /// }
    /// # }
    /// ```
    pub async fn parse_command(
        &self,
        command: &Command,
        context: &CommandContext,
    ) -> Option<CommandResult> {
        let result = match command {
            Command::Help => handle_help(),
            Command::Register(_, _, _) => match handle_register(context, command).await {
                Some(result) => result,
                None => return None,
            },
            Command::Unregister(_) => match handle_unregister(context, command).await {
                Some(result) => result,
                None => return None,
            },
            Command::Games => handle_games(context),
            Command::Alerts => handle_alerts(context),
        };

        Some(result)
    }

    /// Generates a formatted notification message for a player's turn.
    ///
    /// This method creates a user-friendly message to notify players that it's their
    /// turn in a game. The message is formatted as Markdown for display in Matrix.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The Matrix user ID of the player
    /// * `player_url` - The URL to the player's game page
    ///
    /// # Returns
    ///
    /// A formatted string containing the turn notification message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use miou::commands::Commander;
    /// let message = Commander::get_player_turn_message("https://example.com/player?id=p123".to_string());
    /// assert!(message.contains("example.com"));
    /// ```
    pub fn get_player_turn_message(user_id: &str, player_url: &str) -> String {
        format_player_turn(user_id, player_url)
    }

    /// Generates a formatted error message for API access errors.
    ///
    /// This method creates a user-friendly error message to display when the bot
    /// encounters authorization or authentication issues while accessing the
    /// Terraforming Mars API.
    ///
    /// # Returns
    ///
    /// A formatted string containing the access error message.
    ///
    /// # Examples
    ///
    /// ```
    /// # use miou::commands::Commander;
    /// let message = Commander::get_access_error_message();
    /// assert!(message.contains("unauthorized"));
    /// ```
    pub fn get_access_error_message() -> String {
        format_access_error()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::alerts::Alert;
    use crate::tmars::{Game, Phase, Player};

    fn create_test_context() -> CommandContext {
        CommandContext {
            games_map: HashMap::new(),
            alerts_map: HashMap::new(),
            room_id: "!room:example.com".to_string(),
            user_id: "@user:example.com".to_string(),
        }
    }

    fn create_test_game(id: &str) -> Game {
        Game {
            id: id.to_string(),
            phase: Phase::Action,
            spectator_id: "spec123".to_string(),
            players: vec![
                Player {
                    id: "player1".to_string(),
                    name: "Alice".to_string(),
                    color: "red".to_string(),
                    url: "http://example.com/player1".to_string(),
                },
                Player {
                    id: "player2".to_string(),
                    name: "Bob".to_string(),
                    color: "blue".to_string(),
                    url: "http://example.com/player2".to_string(),
                },
            ],
            waited_players: HashSet::new(),
        }
    }

    #[test]
    fn test_parse_valid_help_command() {
        let commander = Commander::new();
        let result = commander.parse("!miou help");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Command::Help));
    }

    #[test]
    fn test_parse_valid_games_command() {
        let commander = Commander::new();
        let result = commander.parse("!miou games");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Command::Games));
    }

    #[test]
    fn test_parse_valid_alerts_command() {
        let commander = Commander::new();
        let result = commander.parse("!miou alerts");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Command::Alerts));
    }

    #[test]
    fn test_parse_valid_register_command() {
        let commander = Commander::new();
        let result = commander.parse("!miou register game123 Alice 60");
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Register(game_id, player_name, delay) => {
                assert_eq!(game_id, "game123");
                assert_eq!(player_name, "Alice");
                assert_eq!(delay, 60);
            }
            _ => panic!("Expected Register command"),
        }
    }

    #[test]
    fn test_parse_valid_unregister_command() {
        let commander = Commander::new();
        let result = commander.parse("!miou unregister game123");
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Unregister(game_id) => {
                assert_eq!(game_id, "game123");
            }
            _ => panic!("Expected Unregister command"),
        }
    }

    #[test]
    fn test_parse_invalid_command_returns_error() {
        let commander = Commander::new();
        let result = commander.parse("!miou unknown_command");
        assert!(result.is_err());
        match result.err().unwrap() {
            CommandParseError::InvalidCommand(msg) => {
                assert!(msg.contains("Unknown command"));
            }
            _ => panic!("Expected InvalidCommand error"),
        }
    }

    #[test]
    fn test_parse_not_for_bot() {
        let commander = Commander::new();
        let result = commander.parse("!other_bot help");
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            CommandParseError::NotForBot
        ));
    }

    #[test]
    fn test_parse_not_a_command() {
        let commander = Commander::new();
        let result = commander.parse("This is just a regular message");
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            CommandParseError::NotForBot
        ));
    }

    #[test]
    fn test_parse_invalid_register_missing_args() {
        let commander = Commander::new();
        let result = commander.parse("!miou register game123");
        assert!(result.is_err());
        match result.err().unwrap() {
            CommandParseError::InvalidCommand(msg) => {
                assert!(msg.contains("Invalid register"));
            }
            _ => panic!("Expected InvalidCommand error"),
        }
    }

    #[test]
    fn test_parse_invalid_register_bad_delay() {
        let commander = Commander::new();
        let result = commander.parse("!miou register game123 Alice invalid");
        assert!(result.is_err());
        match result.err().unwrap() {
            CommandParseError::InvalidCommand(msg) => {
                assert!(msg.contains("Invalid register"));
            }
            _ => panic!("Expected InvalidCommand error"),
        }
    }

    #[test]
    fn test_parse_invalid_unregister_missing_args() {
        let commander = Commander::new();
        let result = commander.parse("!miou unregister");
        assert!(result.is_err());
        match result.err().unwrap() {
            CommandParseError::InvalidCommand(msg) => {
                assert!(msg.contains("Invalid unregister"));
            }
            _ => panic!("Expected InvalidCommand error"),
        }
    }

    #[test]
    fn test_parse_empty_command() {
        let commander = Commander::new();
        let result = commander.parse("!miou");
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), Command::Help));
    }

    #[test]
    fn test_parse_register_with_numeric_player_name() {
        let commander = Commander::new();
        let result = commander.parse("!miou register game123 123 60");
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Register(game_id, player_name, delay) => {
                assert_eq!(game_id, "game123");
                assert_eq!(player_name, "123");
                assert_eq!(delay, 60);
            }
            _ => panic!("Expected Register command"),
        }
    }

    #[test]
    fn test_parse_register_with_zero_delay() {
        let commander = Commander::new();
        let result = commander.parse("!miou register game123 Alice 0");
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Register(_game_id, _player_name, delay) => {
                assert_eq!(delay, 0);
            }
            _ => panic!("Expected Register command"),
        }
    }

    #[test]
    fn test_parse_register_with_large_delay() {
        let commander = Commander::new();
        let result = commander.parse("!miou register game123 Alice 999999");
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Register(_game_id, _player_name, delay) => {
                assert_eq!(delay, 999999);
            }
            _ => panic!("Expected Register command"),
        }
    }

    #[tokio::test]
    async fn test_parse_command_help() {
        let commander = Commander::new();
        let context = create_test_context();
        let command = Command::Help;

        let result = commander.parse_command(&command, &context).await;
        assert!(result.is_some());
        let cmd_result = result.unwrap();
        assert!(!cmd_result.response.is_empty());
        assert!(cmd_result.alert_to_add.is_none());
        assert!(cmd_result.alerts_to_remove.is_none());
    }

    #[tokio::test]
    async fn test_parse_command_games() {
        let commander = Commander::new();
        let mut context = create_test_context();
        context
            .games_map
            .insert("game1".to_string(), create_test_game("game1"));
        let command = Command::Games;

        let result = commander.parse_command(&command, &context).await;
        assert!(result.is_some());
        let cmd_result = result.unwrap();
        assert!(!cmd_result.response.is_empty());
        assert!(cmd_result.alert_to_add.is_none());
        assert!(cmd_result.alerts_to_remove.is_none());
    }

    #[tokio::test]
    async fn test_parse_command_alerts() {
        let commander = Commander::new();
        let mut context = create_test_context();
        let alert = Alert {
            room_id: "!room:example.com".to_string(),
            player_id: "player1".to_string(),
            user_id: "@user:example.com".to_string(),
            player_url: "http://alice.example.com".to_string(),
            notified: false,
            delay: 60,
        };
        let mut alerts_set = HashSet::new();
        alerts_set.insert(alert);
        context.alerts_map.insert("game1".to_string(), alerts_set);

        let command = Command::Alerts;

        let result = commander.parse_command(&command, &context).await;
        assert!(result.is_some());
        let cmd_result = result.unwrap();
        assert!(!cmd_result.response.is_empty());
        assert!(cmd_result.alert_to_add.is_none());
        assert!(cmd_result.alerts_to_remove.is_none());
    }

    #[tokio::test]
    async fn test_parse_command_register_game_not_found() {
        let commander = Commander::new();
        let context = create_test_context();
        let command = Command::Register("nonexistent".to_string(), "Alice".to_string(), 60);

        let result = commander.parse_command(&command, &context).await;
        // Should return Some with error message when game is not found
        assert!(result.is_some());
        let cmd_result = result.unwrap();
        assert!(!cmd_result.response.is_empty());
        assert!(cmd_result.alert_to_add.is_none());
        assert!(cmd_result.alerts_to_remove.is_none());
    }

    #[tokio::test]
    async fn test_parse_command_unregister_no_alerts() {
        let commander = Commander::new();
        let context = create_test_context();
        let command = Command::Unregister("game123".to_string());

        let result = commander.parse_command(&command, &context).await;
        // Should return Some with success message even when there are no alerts
        assert!(result.is_some());
        let cmd_result = result.unwrap();
        assert!(!cmd_result.response.is_empty());
        assert!(cmd_result.alert_to_add.is_none());
        assert!(cmd_result.alerts_to_remove.is_some());
    }

    #[test]
    fn test_get_player_turn_message() {
        assert_eq!(
            Commander::get_player_turn_message("@alice:example.com", "http://example.com/player1"),
            "@alice:example.com: it's your turn to play: [http://example.com/player1](http://example.com/player1)."
        )
    }

    #[test]
    fn test_get_access_error_message() {
        assert_eq!(
            Commander::get_access_error_message(),
            "Error: unauthorized access to the terraforming mars API"
        );
    }
}
