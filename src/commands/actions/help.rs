//! Help command handler.
//!
//! Displays comprehensive help information including all available commands,
//! their syntax, and a brief description of the bot's alert functionality.
//!
//! This is a stateless command that always returns the same help message.

use log::debug;

use crate::commands::{CommandResult, markdown_response::format_help};

/// Returns formatted help information about available commands.
///
/// Generates a Markdown-formatted message listing all bot commands with syntax
/// and usage information. This command is read-only and doesn't modify any state.
pub fn handle_help() -> CommandResult {
    debug!("handling help command");

    CommandResult {
        response: format_help(),
        alert_to_add: None,
        alerts_to_remove: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_help() {
        let result = handle_help();

        // Verify it returns a CommandResult
        assert!(result.alert_to_add.is_none());
        assert!(result.alerts_to_remove.is_none());
        assert!(!result.response.is_empty());
    }
}
