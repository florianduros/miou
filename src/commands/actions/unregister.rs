//! Player unregistration command handler.
//!
//! Removes all alert subscriptions for a specific game in the current Matrix room
//! for the requesting user.
//!
//! # Scope
//!
//! Unregistration is scoped by three parameters:
//! - **Game ID**: Only affects alerts for the specified game
//! - **Room ID**: Only affects alerts in the current room
//! - **User ID**: Only affects alerts belonging to the requesting user
//!
//! # Behavior
//!
//! - Removes all alerts for the game (even if multiple players were registered)
//! - Always returns success (even if no alerts existed)
//! - Doesn't affect alerts in other rooms or other users' alerts

use log::debug;

use crate::commands::{
    CommandContext, CommandResult, command::Command,
    markdown_response::format_successful_unregister,
};

/// Removes all alerts for a game in the current room for the requesting user.
///
/// Extracts the game ID from the command and returns a `CommandResult` indicating
/// which alerts should be removed. The actual removal is performed by the caller.
///
/// # Returns
///
/// - `Some(CommandResult)`: Success message with `alerts_to_remove` containing (game_id, room_id, user_id)
/// - `None`: Only if the command is not an `Unregister` variant
pub async fn handle_unregister(
    context: &CommandContext,
    command: &Command,
) -> Option<CommandResult> {
    debug!("handling unregister command: {:?}", command);

    let game_id = match command {
        Command::Unregister(game_id) => game_id.clone(),
        _ => return None,
    };

    let CommandContext {
        room_id,
        user_id,
        games_map: _,
        alerts_map: _,
    } = context;

    let result = CommandResult {
        response: format_successful_unregister(),
        alert_to_add: None,
        alerts_to_remove: Some((game_id, room_id.clone(), user_id.clone())),
    };

    debug!("unregister command result {:?}", result);

    Some(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn create_test_context() -> CommandContext {
        CommandContext {
            games_map: HashMap::new(),
            alerts_map: HashMap::new(),
            room_id: "!test_room:matrix.org".to_string(),
            user_id: "@test_user:matrix.org".to_string(),
        }
    }

    #[tokio::test]
    async fn test_handle_unregister_successful() {
        let context = create_test_context();
        let command = Command::Unregister("game123".to_string());

        let result = handle_unregister(&context, &command).await;

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.response, format_successful_unregister());
        assert!(result.alert_to_add.is_none());
        assert!(result.alerts_to_remove.is_some());

        let (game_id, room_id, user_id) = result.alerts_to_remove.unwrap();
        assert_eq!(game_id, "game123");
        assert_eq!(room_id, "!test_room:matrix.org");
        assert_eq!(user_id, "@test_user:matrix.org");
    }

    #[tokio::test]
    async fn test_handle_unregister_wrong_command_type_help() {
        let context = create_test_context();
        let command = Command::Help;

        let result = handle_unregister(&context, &command).await;

        assert!(result.is_none());
    }
}
