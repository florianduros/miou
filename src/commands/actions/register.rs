//! Player registration command handler.
//!
//! Registers users to receive Matrix notifications when their turn arrives in
//! Terraforming Mars games. The bot will send a mention after the specified delay.
//!
//! # Validation
//!
//! The handler validates three constraints:
//! - **Delay**: Must be between 1 and 10,080 minutes (1 week)
//! - **Game**: Must exist in the active games map
//! - **Player**: Must be a member of the specified game (case-sensitive)
//!
//! # Errors
//!
//! Returns user-friendly error messages for invalid delay, non-existent games,
//! or players not found in the specified game.

use std::collections::HashMap;

use log::debug;

use crate::{
    alerts::Alert,
    commands::{
        CommandContext, CommandResult,
        command::Command,
        markdown_response::{
            format_game_not_found, format_invalid_delay, format_player_not_found,
            format_successful_register,
        },
    },
    tmars::Game,
};

/// Errors that can occur during player registration.
#[derive(Debug)]
enum RegisterError {
    /// The specified delay is invalid (must be between 1 minute and 1 week).
    InvalidDelay,
    /// The specified game ID does not exist.
    GameNotFound,
    /// The specified player name was not found in the game.
    PlayerNotFound,
}

/// Formats a registration error into a human-readable message.
///
/// Converts a [`RegisterError`] into a formatted Markdown string appropriate
/// for display to the user.
///
/// # Arguments
///
/// * `error` - The registration error that occurred
/// * `game_id` - The game ID that was being registered for
/// * `player_name` - The player name that was being registered
///
/// # Returns
///
/// A formatted error message string.
///
/// # Examples
///
/// ```
/// # use miou::commands::register::{format_register_error, RegisterError};
/// let message = format_register_error(
///     RegisterError::GameNotFound,
///     "game123".to_string(),
///     "Alice".to_string()
/// );
/// ```
fn format_register_error(error: RegisterError, game_id: &str, player_name: &str) -> String {
    match error {
        RegisterError::InvalidDelay => format_invalid_delay(),
        RegisterError::GameNotFound => format_game_not_found(game_id),
        RegisterError::PlayerNotFound => format_player_not_found(player_name, game_id),
    }
}

/// Validates a registration command and retrieves the player ID.
///
/// Performs validation in order:
/// 1. Delay must be between 1 and 10,080 minutes (1 week)
/// 2. Game must exist in the games map
/// 3. Player name must match a player in that game
///
/// # Returns
///
/// - `Ok(String)`: The player ID if all validations pass
/// - `Err(RegisterError)`: Specific error for invalid delay, game not found, or player not found
fn validate_and_get_player(
    (game_id, player_name, delay): (String, String, u64),
    games: &HashMap<String, Game>,
) -> Result<(String, String), RegisterError> {
    // Validate delay (must be between 1 minute and 1 week)
    let week_minutes = 7 * 24 * 60;

    if delay == 0 || delay > week_minutes {
        debug!("invalid delay: {}", delay);
        return Err(RegisterError::InvalidDelay);
    }

    let game = match games.get(&game_id) {
        None => {
            debug!("game {} not found", game_id);
            return Err(RegisterError::GameNotFound);
        }
        Some(g) => g,
    };

    let player = match game.players.iter().find(|p| p.name == player_name) {
        None => {
            debug!("player {} not found in game {}", player_name, game_id);
            return Err(RegisterError::PlayerNotFound);
        }
        Some(p) => (p.id.clone(), p.url.clone()),
    };

    debug!(
        "registered player {} ({}) for game {} with delay {} minutes",
        player_name, player.0, game_id, delay
    );

    Ok(player)
}

/// Registers a user for turn notifications in a game.
///
/// Extracts command parameters, validates them, and returns a `CommandResult` with
/// either a success message and alert to add, or an error message.
///
/// # Returns
///
/// - `Some(CommandResult)`: Always returns a result (success or error message)
/// - `None`: Only if the command is not a `Register` variant
pub async fn handle_register(context: &CommandContext, command: &Command) -> Option<CommandResult> {
    debug!("handling register command: {:?}", command);

    let (game_id, player_name, delay) = match command {
        Command::Register(game_id, player_name, delay) => {
            (game_id.clone(), player_name.clone(), delay)
        }
        _ => return None,
    };

    let CommandContext {
        room_id,
        user_id,
        games_map,
        alerts_map: _,
    } = context;

    let (player_id, player_url) =
        match validate_and_get_player((game_id.clone(), player_name.clone(), *delay), games_map) {
            Err(e) => {
                return Some(CommandResult {
                    response: format_register_error(e, game_id.as_str(), player_name.as_str()),
                    alerts_to_remove: None,
                    alert_to_add: None,
                });
            }
            Ok(id) => id,
        };

    let result = CommandResult {
        response: format_successful_register(),
        alerts_to_remove: None,
        alert_to_add: Some((
            game_id,
            Alert {
                player_id,
                room_id: room_id.clone(),
                user_id: user_id.clone(),
                notified: false,
                delay: *delay,
                player_url,
            },
        )),
    };

    debug!("register command result {:?}", result);

    Some(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::tmars::{Phase, Player};

    use super::*;

    #[test]
    fn test_format_register_error_invalid_delay() {
        assert_eq!(
            format_register_error(RegisterError::InvalidDelay, "game_id", "player_name"),
            format_invalid_delay()
        );
    }

    #[test]
    fn test_format_register_error_game_not_found() {
        assert_eq!(
            format_register_error(RegisterError::GameNotFound, "game_id", "player_name"),
            format_game_not_found("game_id")
        );
    }

    #[test]
    fn test_format_register_error_player_not_found() {
        assert_eq!(
            format_register_error(RegisterError::PlayerNotFound, "game_id", "player_name"),
            format_player_not_found("player_name", "game_id")
        );
    }

    #[test]
    fn test_validate_and_get_player_delay_0() {
        assert!(matches!(
            validate_and_get_player(
                ("game_id".to_string(), "player_name".to_string(), 0),
                &HashMap::new()
            ),
            Err(RegisterError::InvalidDelay)
        ));
    }

    #[test]
    fn test_validate_and_get_player_delay_2_weeks() {
        // 2 weeks
        let weeks = 2 * 7 * 24 * 60;
        assert!(matches!(
            validate_and_get_player(
                ("game_id".to_string(), "player_name".to_string(), weeks),
                &HashMap::new()
            ),
            Err(RegisterError::InvalidDelay)
        ));
    }

    #[test]
    fn test_validate_and_get_player_game_not_found() {
        assert!(matches!(
            validate_and_get_player(
                ("game_id".to_string(), "player_name".to_string(), 1),
                &HashMap::new()
            ),
            Err(RegisterError::GameNotFound)
        ));
    }

    #[test]
    fn test_validate_and_get_player_player_not_found() {
        let game_id = "game_id".to_string();
        let game = Game {
            id: game_id.to_owned(),
            phase: Phase::Research,
            spectator_id: "spectator_id".to_string(),
            players: Vec::new(),
            waited_players: HashSet::new(),
        };

        assert!(matches!(
            validate_and_get_player(
                (game_id.to_owned(), "player_name".to_string(), 1),
                &HashMap::from([(game_id, game)])
            ),
            Err(RegisterError::PlayerNotFound)
        ));
    }

    #[test]
    fn test_validate_and_get_player() {
        let game_id = "game_id".to_string();
        let player_name = "player_name".to_string();
        let player_id = "player_id".to_string();

        let player = Player {
            id: player_id.to_owned(),
            color: "red".to_string(),
            name: player_name.to_owned(),
            url: "http://example.com/player".to_string(),
        };
        let game = Game {
            id: game_id.to_owned(),
            phase: Phase::Research,
            spectator_id: "spectator_id".to_string(),
            players: Vec::from([player]),
            waited_players: HashSet::new(),
        };

        assert_eq!(
            validate_and_get_player(
                (game_id.to_owned(), player_name, 1),
                &HashMap::from([(game_id, game)])
            )
            .unwrap()
            .0,
            player_id
        );
    }

    // Helper function to create a test context
    fn create_test_context(games: Vec<Game>) -> CommandContext {
        let mut games_map = HashMap::new();
        for game in games {
            games_map.insert(game.id.clone(), game);
        }

        CommandContext {
            games_map,
            alerts_map: HashMap::new(),
            room_id: "!test_room:matrix.org".to_string(),
            user_id: "@test_user:matrix.org".to_string(),
        }
    }

    // Helper function to create a test game
    fn create_test_game(id: &str, players: Vec<(&str, &str, &str)>) -> Game {
        Game {
            id: id.to_string(),
            phase: Phase::Action,
            spectator_id: format!("spectator_{}", id),
            players: players
                .iter()
                .map(|(id, name, color)| Player {
                    id: id.to_string(),
                    name: name.to_string(),
                    color: color.to_string(),
                    url: "http://example.com/player".to_string(),
                })
                .collect(),
            waited_players: HashSet::new(),
        }
    }

    #[tokio::test]
    async fn test_handle_register_successful() {
        let game = create_test_game("game1", vec![("player1", "Alice", "red")]);
        let context = create_test_context(vec![game]);
        let command = Command::Register("game1".to_string(), "Alice".to_string(), 60);

        let result = handle_register(&context, &command).await;

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.response, format_successful_register());
        assert!(result.alert_to_add.is_some());
        assert!(result.alerts_to_remove.is_none());

        let (game_id, alert) = result.alert_to_add.unwrap();
        assert_eq!(game_id, "game1");
        assert_eq!(alert.player_id, "player1");
        assert_eq!(alert.user_id, "@test_user:matrix.org");
        assert_eq!(alert.room_id, "!test_room:matrix.org");
        assert_eq!(alert.delay, 60);
        assert!(!alert.notified);
    }

    #[tokio::test]
    async fn test_handle_register_invalid_delay_zero() {
        let game = create_test_game("game1", vec![("player1", "Alice", "red")]);
        let context = create_test_context(vec![game]);
        let command = Command::Register("game1".to_string(), "Alice".to_string(), 0);

        let result = handle_register(&context, &command).await;

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.response, format_invalid_delay());
        assert!(result.alert_to_add.is_none());
        assert!(result.alerts_to_remove.is_none());
    }

    #[tokio::test]
    async fn test_handle_register_game_not_found() {
        let context = create_test_context(vec![]);
        let command = Command::Register("game999".to_string(), "Alice".to_string(), 60);

        let result = handle_register(&context, &command).await;

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.response, format_game_not_found("game999"));
        assert!(result.alert_to_add.is_none());
        assert!(result.alerts_to_remove.is_none());
    }

    #[tokio::test]
    async fn test_handle_register_player_not_found() {
        let game = create_test_game("game1", vec![("player1", "Alice", "red")]);
        let context = create_test_context(vec![game]);
        let command = Command::Register("game1".to_string(), "Bob".to_string(), 60);

        let result = handle_register(&context, &command).await;

        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.response, format_player_not_found("Bob", "game1"));
        assert!(result.alert_to_add.is_none());
        assert!(result.alerts_to_remove.is_none());
    }

    #[tokio::test]
    async fn test_handle_register_wrong_command_type() {
        let game = create_test_game("game1", vec![("player1", "Alice", "red")]);
        let context = create_test_context(vec![game]);
        let command = Command::Help;

        let result = handle_register(&context, &command).await;

        assert!(result.is_none());
    }
}
