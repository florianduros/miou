//! Command parsing and handling.
//!
//! This module provides command parsing functionality for the bot, converting
//! Matrix message text into structured [`Command`] enums that can be processed
//! by the application.

use command_parser::{Command as ParserCommand, Parser};
use log::debug;

use crate::commands::markdown_response::{
    format_invalid_register, format_invalid_unregister, format_unknown_command,
};

/// Represents a parsed bot command.
///
/// Commands are parsed from Matrix message text and represent the various
/// operations users can perform with the bot.
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Command {
    /// Display help information
    Help,
    /// List all ongoing games
    Games,
    /// Register for game alerts
    ///
    /// # Fields
    ///
    /// * `String` - Game ID
    /// * `String` - Player name
    /// * `u32` - Delay in minutes
    Register(String, String, u64),
    /// Unregister from game alerts
    ///
    /// # Fields
    ///
    /// * `String` - Game ID
    Unregister(String),
    /// List user's registered alerts
    Alerts,
}

/// Errors that can occur during command parsing.
#[derive(Debug)]
pub enum CommandParsingError {
    /// The message could not be parsed as a command
    UnableToParse,
    /// The command is not for this bot (wrong prefix)
    NotMiou,
    /// The command is not recognized
    Unknown,
    /// The register command has invalid syntax or arguments
    InvalidRegister,
    /// The unregister command has invalid syntax or arguments
    InvalidUnRegister,
}

impl Command {
    /// Parses a message string into a Command.
    ///
    /// This method attempts to parse a Matrix message body into a structured
    /// command. It handles the bot prefix check and validates command syntax.
    ///
    /// # Arguments
    ///
    /// * `parser` - The command parser instance configured for the bot
    /// * `body` - The message text to parse
    ///
    /// # Returns
    ///
    /// * `Ok(Command)` - If the message is a valid bot command
    /// * `Err(CommandParsingError)` - If parsing fails or the command is invalid
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The message is not a command format - [`CommandParsingError::UnableToParse`]
    /// - The command is for a different bot - [`CommandParsingError::NotMiou`]
    /// - The command is not recognized - [`CommandParsingError::Unknown`]
    /// - Register command has invalid arguments - [`CommandParsingError::InvalidRegister`]
    /// - Unregister command has invalid arguments - [`CommandParsingError::InvalidUnRegister`]
    ///
    /// # Examples
    ///
    /// ```
    /// # use command_parser::Parser;
    /// # use miou::commands::command::Command;
    /// let parser = Parser::new('!', '-')
    ///     .command("miou")
    ///     .subcommand("help")
    ///     .build();
    /// let result = Command::parse(&parser, "!miou help".to_string());
    /// assert!(result.is_ok());
    /// ```
    pub fn parse(parser: &Parser, body: &str) -> Result<Self, CommandParsingError> {
        // For an unknown reason the parser ignores the last word, so we add a dummy word at the end
        let body = body.to_string() + " dummy";

        // This is normal to fails if the message is not a command
        let command = match parser.parse(&body) {
            Ok(cmd) => cmd,
            Err(_) => return Err(CommandParsingError::UnableToParse),
        };

        // Ignore commands that are not for the bot
        if command.name != "miou" {
            return Err(CommandParsingError::NotMiou);
        }

        debug!("Parsing command: {:?}", command);

        // If no arguments, return help
        if command.arguments.is_empty() {
            return Ok(Command::Help);
        }

        match command.arguments[0].as_str() {
            "help" => Ok(Command::Help),
            "games" => Ok(Command::Games),
            "register" => {
                let (game_id, player_name, delay) = Self::parse_register(&command)?;
                Ok(Command::Register(game_id, player_name, delay))
            }
            "alerts" => Ok(Command::Alerts),
            "unregister" => Ok(Command::Unregister(Self::parse_unregister(&command)?)),
            _ => Err(CommandParsingError::Unknown),
        }
    }

    fn parse_register(
        command: &ParserCommand,
    ) -> Result<(String, String, u64), CommandParsingError> {
        debug!("Parsing register command: {:?}", command);

        // 4 arguments: register, game id, player name and delay
        if command.arguments.len() < 4 {
            return Err(CommandParsingError::InvalidRegister);
        }

        let game_id = command.arguments[1].clone();
        let player_name = command.arguments[2].clone();
        let delay = match command.arguments[3].parse::<u64>() {
            Ok(delay) => delay,
            Err(_) => return Err(CommandParsingError::InvalidRegister),
        };

        debug!(
            "Parsed register command - game_id: {}, player_name: {}, delay: {}",
            game_id, player_name, delay
        );

        Ok((game_id, player_name, delay))
    }

    fn parse_unregister(command: &ParserCommand) -> Result<String, CommandParsingError> {
        debug!("Parsing unregister command: {:?}", command);

        // 2 arguments: unregister and game id
        if command.arguments.len() < 2 {
            return Err(CommandParsingError::InvalidUnRegister);
        }

        let game_id = command.arguments[1].clone();

        debug!("Parsed unregister command - game_id: {}", game_id);

        Ok(game_id)
    }
}

/// Formats a command error into a user-friendly message.
///
/// Converts certain [`CommandParsingError`] variants into formatted error messages
/// for display to the user. Not all errors produce messages (e.g., `UnableToParse`
/// and `NotMiou` return `None` to avoid responding to non-command messages).
///
/// # Arguments
///
/// * `error` - The command error to format
///
/// # Returns
///
/// * `Some(String)` - A formatted error message for user-facing errors
/// * `None` - For internal errors that should not produce a response
///
/// # Examples
///
/// ```
/// # use miou::commands::command::{format_command_error, CommandParsingError};
/// let error = CommandParsingError::Unknown;
/// let message = format_command_error(error);
/// assert!(message.is_some());
/// ```
pub fn format_command_error(error: CommandParsingError) -> Option<String> {
    match error {
        CommandParsingError::Unknown => Some(format_unknown_command()),
        CommandParsingError::InvalidRegister => Some(format_invalid_register()),
        CommandParsingError::InvalidUnRegister => Some(format_invalid_unregister()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_parser() -> Parser {
        Parser::new('!', '-')
    }

    #[test]
    fn test_parse_help_command() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou help");
        assert!(matches!(result, Ok(Command::Help)));
    }

    #[test]
    fn test_parse_help_command_no_args() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou");
        assert!(matches!(result, Ok(Command::Help)));
    }

    #[test]
    fn test_parse_games_command() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou games");
        assert!(matches!(result, Ok(Command::Games)));
    }

    #[test]
    fn test_parse_alerts_command() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou alerts");
        assert!(matches!(result, Ok(Command::Alerts)));
    }

    #[test]
    fn test_parse_register_command() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou register game123 Alice 60");
        assert!(matches!(
            result,
            Ok(Command::Register(game_id, player_name, delay))
            if game_id == "game123" && player_name == "Alice" && delay == 60
        ));
    }

    #[test]
    fn test_parse_register_command_invalid_missing_args() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou register game123");
        assert!(matches!(result, Err(CommandParsingError::InvalidRegister)));
    }

    #[test]
    fn test_parse_register_command_invalid_delay() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou register game123 Alice invalid");
        assert!(matches!(result, Err(CommandParsingError::InvalidRegister)));
    }

    #[test]
    fn test_parse_unregister_command() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou unregister game123");
        assert!(matches!(
            result,
            Ok(Command::Unregister(game_id)) if game_id == "game123"
        ));
    }

    #[test]
    fn test_parse_unregister_command_invalid_missing_args() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou unregister");
        assert!(matches!(
            result,
            Err(CommandParsingError::InvalidUnRegister)
        ));
    }

    #[test]
    fn test_parse_unknown_command() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!miou unknown");
        assert!(matches!(result, Err(CommandParsingError::Unknown)));
    }

    #[test]
    fn test_parse_not_miou_command() {
        let parser = create_parser();
        let result = Command::parse(&parser, "!other_bot help");
        assert!(matches!(result, Err(CommandParsingError::NotMiou)));
    }

    #[test]
    fn test_parse_unable_to_parse() {
        let parser = create_parser();
        let result = Command::parse(&parser, "This is not a command");
        assert!(matches!(result, Err(CommandParsingError::UnableToParse)));
    }

    #[test]
    fn test_format_command_error_unknown() {
        let error = CommandParsingError::Unknown;
        let result = format_command_error(error);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Unknown command"));
    }

    #[test]
    fn test_format_command_error_invalid_register() {
        let error = CommandParsingError::InvalidRegister;
        let result = format_command_error(error);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Invalid register"));
    }

    #[test]
    fn test_format_command_error_invalid_unregister() {
        let error = CommandParsingError::InvalidUnRegister;
        let result = format_command_error(error);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Invalid unregister"));
    }

    #[test]
    fn test_format_command_error_unable_to_parse() {
        let error = CommandParsingError::UnableToParse;
        let result = format_command_error(error);
        assert!(result.is_none());
    }

    #[test]
    fn test_format_command_error_not_miou() {
        let error = CommandParsingError::NotMiou;
        let result = format_command_error(error);
        assert!(result.is_none());
    }
}
