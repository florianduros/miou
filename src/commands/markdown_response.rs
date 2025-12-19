//! Markdown response formatters for bot commands.
//!
//! This module provides functions to format bot responses in Markdown format
//! for display in Matrix chat rooms. All responses are designed to be user-friendly
//! and informative.

use crate::tmars::Game;

/// Formats the help message showing available bot commands.
///
/// Returns a comprehensive help message listing all available commands,
/// their usage syntax, and a brief description of the bot's functionality.
///
/// # Returns
///
/// A Markdown-formatted string containing the help message.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_help;
/// let help = format_help();
/// assert!(help.contains("Commands:"));
/// ```
pub fn format_help() -> String {
    let body = "Commands:\n\
        - `games`: list all the ongoing games\n\
        - `alerts`: list your registered alerts\n\
        - `register <game_id> <player_name> <delay_in_minutes>`: register a new alert\n\
        - `unregister <game_id>`: unregister an alert\n\
        - `help`: show this help message\n\n\
        Alert sends a mention to the registered user when their turn to play arrives, following the delay set in the register argument.\n\
        > *miou* is a free open source terraforming mars bot. Source code is available on [Github](https://github.com/florianduros/miou).";

    body.to_owned()
}

/// Formats a response for an unknown command.
///
/// Returns a helpful message indicating the command was not recognized
/// and suggests using the help command.
///
/// # Returns
///
/// A Markdown-formatted string for unknown command errors.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_unknown_command;
/// let msg = format_unknown_command();
/// assert!(msg.contains("Unknown command"));
/// ```
pub fn format_unknown_command() -> String {
    "Unknown command. Type `!miou help` for more information.".to_owned()
}

/// Formats a list of ongoing games.
///
/// Displays all ongoing games with their IDs, current phase, and player lists.
/// Players who are currently being waited for are marked with an hourglass emoji (⏳).
///
/// # Arguments
///
/// * `games` - A slice of [`Game`] structs representing the ongoing games
///
/// # Returns
///
/// A Markdown-formatted string listing all games, or a message if no games exist.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_games;
/// # use miou::tmars::structs::Game;
/// let games: Vec<Game> = vec![];
/// let output = format_games(&games);
/// assert_eq!(output, "No ongoing games found.");
/// ```
pub fn format_games(games: &[Game]) -> String {
    if games.is_empty() {
        return "No ongoing games found.".to_owned();
    }

    let games_md = games
        .iter()
        .map(|game| {
            // Format players list
            let players = game
                .players
                .iter()
                // Indicate waited players with an hourglass emoji
                .map(|p| match game.waited_players.contains(&p.id) {
                    true => format!("{}(⏳)", p.name.clone()),
                    false => p.name.clone(),
                })
                .collect::<Vec<String>>()
                .join(", ");

            format!(
                "- **{}**({:?}), **players**: {}",
                game.id, game.phase, players
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    format!("Games: \n\n {}", games_md)
}

/// Formats an error response for invalid register command syntax.
///
/// # Returns
///
/// A Markdown-formatted string with the correct register command usage.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_invalid_register;
/// let msg = format_invalid_register();
/// assert!(msg.contains("Usage:"));
/// ```
pub fn format_invalid_register() -> String {
    "Invalid register command. Usage: `!miou register <game_id> <player_name> <delay_in_minutes>`"
        .to_owned()
}

/// Formats an error response for invalid unregister command syntax.
///
/// # Returns
///
/// A Markdown-formatted string with the correct unregister command usage.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_invalid_unregister;
/// let msg = format_invalid_unregister();
/// assert!(msg.contains("Usage:"));
/// ```
pub fn format_invalid_unregister() -> String {
    "Invalid unregister command. Usage: `!miou unregister <game_id>`".to_owned()
}

/// Formats an error response for invalid delay values.
///
/// Returned when the delay is outside the valid range (1 minute to 1 week).
///
/// # Returns
///
/// A Markdown-formatted string explaining the delay constraint.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_invalid_delay;
/// let msg = format_invalid_delay();
/// assert!(msg.contains("Invalid delay"));
/// ```
pub fn format_invalid_delay() -> String {
    "Invalid delay. Delay must be between 1 minutes and 1 week.".to_owned()
}

/// Formats an error response when a game is not found.
///
/// # Arguments
///
/// * `game_id` - The ID of the game that was not found
///
/// # Returns
///
/// A Markdown-formatted string indicating the game was not found.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_game_not_found;
/// let msg = format_game_not_found("game123".to_string());
/// assert!(msg.contains("game123"));
/// ```
pub fn format_game_not_found(game_id: &str) -> String {
    format!("Game with id '{}' not found.", game_id)
}

/// Formats an error response when a player is not found in a game.
///
/// # Arguments
///
/// * `player_name` - The name of the player that was not found
/// * `game_id` - The ID of the game where the player was searched for
///
/// # Returns
///
/// A Markdown-formatted string indicating the player was not found in the game.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_player_not_found;
/// let msg = format_player_not_found("Alice".to_string(), "game123".to_string());
/// assert!(msg.contains("Alice"));
/// assert!(msg.contains("game123"));
/// ```
pub fn format_player_not_found(player_name: &str, game_id: &str) -> String {
    format!(
        "Player '{}' not found in game with id '{}'.",
        player_name, game_id
    )
}

/// Formats a success response for player registration.
///
/// # Returns
///
/// A Markdown-formatted string confirming successful registration.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_successful_register;
/// let msg = format_successful_register();
/// assert!(msg.contains("registered successfully"));
/// ```
pub fn format_successful_register() -> String {
    "You have been registered successfully.".to_owned()
}

/// Formats a success response for player unregistration.
///
/// # Returns
///
/// A Markdown-formatted string confirming successful unregistration.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_successful_unregister;
/// let msg = format_successful_unregister();
/// assert!(msg.contains("unregistered successfully"));
/// ```
pub fn format_successful_unregister() -> String {
    "You have been unregistered successfully.".to_owned()
}

/// Formats a notification message for a player's turn.
///
/// Creates a message notifying the user that it's their turn to play,
/// including the URL to access their game.
///
/// # Arguments
///
/// * `user_id` - The Matrix user ID of the player
/// * `player_url` - The URL to the player's game page on the Terraforming Mars server
///
/// # Returns
///
/// A Markdown-formatted string with the turn notification message.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_player_turn;
/// let msg = format_player_turn("https://example.com/player?id=p123".to_string());
/// assert!(msg.contains("turn to play"));
/// ```
pub fn format_player_turn(user_id: &str, player_url: &str) -> String {
    format!(
        "{}: it's your turn to play: [{}]({}).",
        user_id, player_url, player_url
    )
}

/// Formats a list of registered alerts for the user.
///
/// Displays all alerts grouped by game ID, showing which players are being
/// monitored in each game.
///
/// # Arguments
///
/// * `alerts` - A slice of tuples containing the game ID and a vector of player names
///
/// # Returns
///
/// A Markdown-formatted string listing all registered alerts, or a message if no alerts exist.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_alerts;
/// let alerts = vec![
///     ("game1".to_string(), vec!["Alice".to_string()]),
/// ];
/// let output = format_alerts(&alerts);
/// assert!(output.contains("game1"));
/// ```
pub fn format_alerts(alerts: &[(String, Vec<String>)]) -> String {
    if alerts.is_empty() {
        return "No alerts found.".to_owned();
    }

    let alerts_md = alerts
        .iter()
        .map(|(game_id, player_name)| format!("- {}: {}", game_id, player_name.join(", ")))
        .collect::<Vec<String>>()
        .join("\n");

    format!("Registered alerts:\n\n {}", alerts_md)
}

/// Formats an error message for API access failures.
///
/// Returns an error message indicating that the bot encountered authorization
/// or authentication issues while trying to access the Terraforming Mars API.
///
/// # Returns
///
/// A Markdown-formatted string containing the access error message.
///
/// # Examples
///
/// ```
/// # use miou::commands::markdown_response::format_access_error;
/// let msg = format_access_error();
/// assert!(msg.contains("unauthorized"));
/// assert!(msg.contains("terraforming mars"));
/// ```
pub fn format_access_error() -> String {
    "Error: unauthorized access to the terraforming mars API".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmars::{Phase, Player};
    use std::collections::HashSet;

    #[test]
    fn test_format_help() {
        let help = format_help();
        assert!(help.contains("Commands:"));
        assert!(help.contains("games"));
        assert!(help.contains("alerts"));
        assert!(help.contains("register"));
        assert!(help.contains("unregister"));
        assert!(help.contains("help"));
    }

    #[test]
    fn test_format_unknown_command() {
        assert_eq!(
            format_unknown_command(),
            "Unknown command. Type `!miou help` for more information.",
        );
    }

    #[test]
    fn test_format_games_empty() {
        assert_eq!(format_games(&[]), "No ongoing games found.",);
    }

    #[test]
    fn test_format_games() {
        let games = [
            Game {
                id: "game-id1".to_owned(),
                phase: Phase::Research,
                spectator_id: "spec-id1".to_owned(),
                players: vec![
                    Player {
                        id: "player-id1".to_owned(),
                        color: "red".to_owned(),
                        name: "Alice".to_owned(),
                        url: "http://example.com/player-id1".to_owned(),
                    },
                    Player {
                        id: "player-id2".to_owned(),
                        color: "blue".to_owned(),
                        name: "Bob".to_owned(),
                        url: "http://example.com/player-id2".to_owned(),
                    },
                ],
                waited_players: HashSet::from(["player-id2".to_owned()]),
            },
            Game {
                id: "game-id2".to_owned(),
                phase: Phase::Research,
                spectator_id: "spec-id2".to_owned(),
                players: vec![Player {
                    id: "player-id3".to_owned(),
                    color: "red".to_owned(),
                    name: "Alice".to_owned(),
                    url: "http://example.com/player-id3".to_owned(),
                }],
                waited_players: HashSet::new(),
            },
        ];

        assert_eq!(
            format_games(&games),
            "Games: \n\n - **game-id1**(Research), **players**: Alice, Bob(⏳)\n- **game-id2**(Research), **players**: Alice",
        );
    }

    #[test]
    fn test_format_invalid_register() {
        assert_eq!(
            format_invalid_register(),
            "Invalid register command. Usage: `!miou register <game_id> <player_name> <delay_in_minutes>`",
        );
    }

    #[test]
    fn test_format_invalid_unregister() {
        assert_eq!(
            format_invalid_unregister(),
            "Invalid unregister command. Usage: `!miou unregister <game_id>`",
        );
    }

    #[test]
    fn test_format_invalid_delay() {
        assert_eq!(
            format_invalid_delay(),
            "Invalid delay. Delay must be between 1 minutes and 1 week.",
        );
    }

    #[test]
    fn test_format_game_not_found() {
        assert_eq!(
            format_game_not_found("game123"),
            "Game with id 'game123' not found.",
        );
    }

    #[test]
    fn test_format_player_not_found() {
        assert_eq!(
            format_player_not_found("Alice", "game123"),
            "Player 'Alice' not found in game with id 'game123'.",
        );
    }

    #[test]
    fn test_format_successful_register() {
        assert_eq!(
            format_successful_register(),
            "You have been registered successfully.",
        );
    }

    #[test]
    fn test_format_alerts_empty() {
        assert_eq!(format_alerts(&[]), "No alerts found.",);
    }

    #[test]
    fn test_format_alerts() {
        let alerts = vec![
            (
                "game-id1".to_owned(),
                vec!["Alice".to_owned(), "Bob".to_owned()],
            ),
            ("game-id2".to_owned(), vec!["Charlie".to_owned()]),
        ];

        assert_eq!(
            format_alerts(&alerts),
            "Registered alerts:\n\n - game-id1: Alice, Bob\n- game-id2: Charlie",
        );
    }

    #[test]
    fn test_format_player_turn() {
        assert_eq!(
            format_player_turn("@alice:example.com", "http://example.com/player-id1"),
            "@alice:example.com: it's your turn to play: [http://example.com/player-id1](http://example.com/player-id1)."
        )
    }

    #[test]
    fn test_format_successful_unregister() {
        assert_eq!(
            format_successful_unregister(),
            "You have been unregistered successfully.",
        );
    }

    #[test]
    fn test_format_access_error() {
        assert_eq!(
            format_access_error(),
            "Error: unauthorized access to the terraforming mars API",
        );
    }
}
