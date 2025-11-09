//! Response structures for TMars API endpoints.
//!
//! This module contains structures for deserializing JSON responses from
//! the Terraforming Mars server API.

use serde::Deserialize;
use std::fmt;

/// Representation of a game from `/api/games`.
///
/// This is a minimal game representation used in the games list endpoint.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameResponse {
    /// Unique identifier for the game.
    pub game_id: String,
}

impl fmt::Display for GameResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "game-id={}", self.game_id)
    }
}

/// Detailed representation of a game from `/api/game?id={gameId}`.
///
/// Contains comprehensive information about a specific game including
/// its phase, players, and spectator ID.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameDetail {
    /// Unique identifier for the game.
    pub id: String,
    /// Current phase of the game.
    pub phase: String,
    /// Spectator identifier for the game.
    pub spectator_id: String,
    /// List of players in the game.
    pub players: Vec<PlayerDetail>,
}

impl fmt::Display for GameDetail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "id={}, phase={}, spectator_id={}, players={:?}",
            self.id, self.phase, self.spectator_id, self.players
        )
    }
}

/// Representation of a player in a game from `/api/game?id={gameId}`.
///
/// Contains player identification and visual display information.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDetail {
    /// Unique identifier for the player.
    pub id: String,
    /// Player's display name.
    pub name: String,
    /// Player's color.
    pub color: String,
}

impl fmt::Display for PlayerDetail {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "id={}, name={}, color={}",
            self.id, self.name, self.color
        )
    }
}

/// Response from `/api/waitingfor?id={spectatorId}`.
///
/// Contains the list of player colors that are currently expected to take action.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WaitingForResponse {
    /// List of player colors who are being waited for.
    pub waiting_for: Vec<String>,
}

impl fmt::Display for WaitingForResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "waiting_for={:?}", self.waiting_for)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_response_display() {
        let game = GameResponse {
            game_id: "game123".to_string(),
        };

        assert_eq!(format!("{}", game), "game-id=game123");
    }

    #[test]
    fn test_game_detail_display() {
        let game = GameDetail {
            id: "game456".to_string(),
            phase: "action".to_string(),
            spectator_id: "spec789".to_string(),
            players: vec![PlayerDetail {
                id: "p1".to_string(),
                name: "Player One".to_string(),
                color: "red".to_string(),
            }],
        };

        let display = format!("{}", game);
        assert!(display.contains("id=game456"));
        assert!(display.contains("phase=action"));
        assert!(display.contains("spectator_id=spec789"));
    }

    #[test]
    fn test_player_detail_display() {
        let player = PlayerDetail {
            id: "p123".to_string(),
            name: "Alice".to_string(),
            color: "green".to_string(),
        };

        assert_eq!(format!("{}", player), "id=p123, name=Alice, color=green");
    }

    #[test]
    fn test_waiting_for_response_display() {
        let response = WaitingForResponse {
            waiting_for: vec!["red".to_string(), "blue".to_string()],
        };

        let display = format!("{}", response);
        assert!(display.contains("waiting_for="));
        assert!(display.contains("red"));
        assert!(display.contains("blue"));
    }

    #[test]
    fn test_game_detail_with_multiple_players() {
        let json = r#"{
            "id": "game777",
            "phase": "production",
            "spectatorId": "spec888",
            "players": [
                {"id": "p1", "name": "Alice", "color": "red"},
                {"id": "p2", "name": "Bob", "color": "blue"},
                {"id": "p3", "name": "Charlie", "color": "green"},
                {"id": "p4", "name": "Diana", "color": "yellow"}
            ]
        }"#;

        let game: GameDetail = serde_json::from_str(json).unwrap();

        assert_eq!(game.players.len(), 4);
        assert_eq!(game.players[2].name, "Charlie");
        assert_eq!(game.players[3].color, "yellow");
    }
}
