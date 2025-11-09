//! Games command handler.
//!
//! Lists all active Terraforming Mars games with the following information:
//! - Game IDs and current phase (Research, Action, etc.)
//! - All players in each game
//! - Players currently waiting for their turn (marked with ⏳)
//!
//! This is a read-only command that queries the games map from the context.

use log::debug;

use crate::commands::{CommandContext, CommandResult, markdown_response::format_games};

/// Lists all active games with their details.
///
/// Retrieves all games from the context's games map and formats them into a
/// Markdown response. Returns "No ongoing games found." if the games map is empty.
pub fn handle_games(context: &CommandContext) -> CommandResult {
    debug!("handling games command");

    let result = CommandResult {
        response: format_games(&context.games_map.values().cloned().collect::<Vec<_>>()),
        alert_to_add: None,
        alerts_to_remove: None,
    };

    debug!("games command result {:?}", result);

    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::tmars::{Game, Phase, Player};

    use super::*;

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

    fn create_test_game(
        id: &str,
        phase: Phase,
        players: Vec<(&str, &str, &str)>,
        waited_player_ids: Vec<&str>,
    ) -> Game {
        let mut waited_players = std::collections::HashSet::new();
        for player_id in waited_player_ids {
            waited_players.insert(player_id.to_string());
        }

        Game {
            id: id.to_string(),
            phase,
            spectator_id: format!("spectator_{}", id),
            players: players
                .iter()
                .map(|(id, name, color)| Player {
                    id: id.to_string(),
                    name: name.to_string(),
                    color: color.to_string(),
                    url: "http://alice.example.com".to_string(),
                })
                .collect(),
            waited_players,
        }
    }

    #[test]
    fn test_handle_games_empty_games_map() {
        let context = create_test_context(vec![]);
        let result = handle_games(&context);

        assert_eq!(result.response, "No ongoing games found.");
        assert_eq!(result.alert_to_add, None);
        assert_eq!(result.alerts_to_remove, None);
    }

    #[test]
    fn test_handle_games_multiple_games() {
        let game1 = create_test_game(
            "game1",
            Phase::Action,
            vec![("player1", "Alice", "red")],
            vec![],
        );

        let game2 = create_test_game(
            "game2",
            Phase::Research,
            vec![("player3", "Charlie", "green")],
            vec![],
        );

        let context = create_test_context(vec![game1, game2]);
        let result = handle_games(&context);

        assert!(result.response.contains("game1"));
        assert!(result.response.contains("game2"));
        assert!(result.response.contains("Alice"));
        assert!(result.response.contains("Charlie"));

        assert_eq!(result.alert_to_add, None);
        assert_eq!(result.alerts_to_remove, None);
    }
}
