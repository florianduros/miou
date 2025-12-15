//! Alerts command handler.
//!
//! Lists all registered alerts for the requesting user in the current Matrix room.
//!
//! Alerts are filtered by:
//! - User ID: Only shows alerts belonging to the requesting user
//! - Room ID: Only shows alerts registered in the current room
//!
//! Player IDs in alerts are resolved to human-readable names using the games map.

use std::collections::HashMap;

use log::debug;

use crate::{
    commands::{CommandContext, CommandResult, markdown_response::format_alerts},
    tmars::Game,
};

/// Lists all alerts for the requesting user in the current room.
///
/// Iterates through all alerts in the alerts map, filters by user ID and room ID,
/// then resolves player IDs to names using the games map. Results are grouped by
/// game ID.
///
/// # Returns
///
/// A `CommandResult` with:
/// - `response`: Markdown-formatted list of alerts grouped by game, or "No alerts found."
/// - `alert_to_add`: Always `None` (read-only command)
/// - `alerts_to_remove`: Always `None` (read-only command)
pub fn handle_alerts(context: &CommandContext) -> CommandResult {
    debug!("handling alerts command");

    let CommandContext {
        room_id,
        user_id,
        games_map,
        alerts_map,
    } = context;

    let mut filtered_alerts: Vec<(String, Vec<String>)> = Vec::new();

    // Filter alerts for the given user_id and room_id
    // First iterates over each game_id and its corresponding alert set
    for (game_id, alert_set) in alerts_map.iter() {
        let mut player_names = Vec::new();

        // Search for alerts matching the user_id and room_id
        // and get the player names
        for alert in alert_set.iter() {
            if alert.user_id == user_id.clone()
                && alert.room_id == room_id.clone()
                && let Some(name) = search_player_name(games_map, game_id, alert.player_id.as_str())
            {
                player_names.push(name);
            }
        }

        // Only keep entries with non-empty player names
        if !player_names.is_empty() {
            filtered_alerts.push((game_id.clone(), player_names));
        }
    }

    let result = CommandResult {
        response: format_alerts(&filtered_alerts),
        alert_to_add: None,
        alerts_to_remove: None,
    };

    debug!("alerts command result {:?}", result);

    result
}

/// Looks up a player's name by ID in a specific game.
///
/// Returns `None` if the game doesn't exist or the player is not found in that game.
fn search_player_name(
    games_map: &HashMap<String, Game>,
    game_id: &str,
    player_id: &str,
) -> Option<String> {
    games_map
        // For the given game, we look for the player name for the player_id
        .get(&game_id.to_owned())
        .and_then(|game| {
            game.players
                .iter()
                .find(|player| player.id == player_id)
                .map(|player| player.name.clone())
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{
        alerts::Alert,
        tmars::{Game, Phase, Player},
    };

    use super::*;

    fn create_test_context(
        games: Vec<Game>,
        alerts: Vec<(String, Vec<Alert>)>,
        room_id: &str,
        user_id: &str,
    ) -> CommandContext {
        let mut games_map = HashMap::new();
        for game in games {
            games_map.insert(game.id.clone(), game);
        }

        let mut alerts_map = HashMap::new();
        for (game_id, alert_vec) in alerts {
            let alert_set: HashSet<Alert> = alert_vec.into_iter().collect();
            alerts_map.insert(game_id, alert_set);
        }

        CommandContext {
            games_map,
            alerts_map,
            room_id: room_id.to_string(),
            user_id: user_id.to_string(),
        }
    }

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
                    url: "http://alice.example.com".to_string(),
                })
                .collect(),
            waited_players: HashSet::new(),
        }
    }

    fn create_test_alert(room_id: &str, user_id: &str, player_id: &str, delay: u64) -> Alert {
        Alert {
            room_id: room_id.to_string(),
            user_id: user_id.to_string(),
            player_id: player_id.to_string(),
            player_url: "http://alice.example.com".to_string(),
            notified: false,
            delay,
        }
    }

    #[test]
    fn test_handle_alerts() {
        let game1 = create_test_game(
            "game1",
            vec![("player1", "Alice", "red"), ("player2", "Bob", "blue")],
        );
        let game2 = create_test_game("game2", vec![("player3", "Charlie", "green")]);

        let alert1 = create_test_alert("!room:matrix.org", "@alice:matrix.org", "player1", 60);
        let alert2 = create_test_alert("!room:matrix.org", "@charlie:matrix.org", "player3", 60);

        let context = create_test_context(
            vec![game1, game2],
            vec![
                ("game1".to_string(), vec![alert1]),
                ("game2".to_string(), vec![alert2]),
            ],
            "!room:matrix.org",
            "@alice:matrix.org",
        );

        let result = handle_alerts(&context);

        println!("result {:?}", result);

        assert_eq!(result.alert_to_add, None);
        assert_eq!(result.alerts_to_remove, None);
        assert!(result.response.contains("Alice"));
        assert!(!result.response.contains("Charlie"));
        assert!(!result.response.contains("Bob"));
    }

    #[test]
    fn test_search_player_name() {
        let game = create_test_game("game1", vec![("player1", "Alice", "red")]);
        let mut games_map = HashMap::new();
        games_map.insert("game1".to_string(), game);

        let result = search_player_name(&games_map, "game1", "player1");

        assert_eq!(result, Some("Alice".to_string()));
    }

    #[test]
    fn test_search_player_name_game_not_found() {
        let game = create_test_game("game1", vec![("player1", "Alice", "red")]);
        let mut games_map = HashMap::new();
        games_map.insert("game1".to_string(), game);

        let result = search_player_name(&games_map, "game999", "player1");

        assert_eq!(result, None);
    }

    #[test]
    fn test_search_player_name_player_not_found() {
        let game = create_test_game("game1", vec![("player1", "Alice", "red")]);
        let mut games_map = HashMap::new();
        games_map.insert("game1".to_string(), game);

        let result = search_player_name(&games_map, "game1", "player999");

        assert_eq!(result, None);
    }
}
