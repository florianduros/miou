//! Internal data structures for representing TMars game state.
//!
//! This module defines the core data structures used internally to represent
//! Terraforming Mars games, players, and game phases.

use std::{collections::HashSet, fmt};

/// Represents a game with its complete state information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    /// Unique identifier for the game
    pub id: String,
    /// Current generation of the game
    pub phase: Phase,
    /// Unique identifier for spectator
    pub spectator_id: String,
    /// Collection of player's ids in the game
    pub players: Vec<Player>,
    /// Player's ids where its they turn to play
    pub waited_players: HashSet<String>,
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "id={}, phase={:?}, spectator_id={}, players={:?}, waited_players={:?}",
            self.id, self.phase, self.spectator_id, self.players, self.waited_players
        )
    }
}

/// Represents a player in a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    /// Unique identifier for the player
    pub id: String,
    /// Secondary identifier (color name)
    ///
    /// Used to identify the player in `/api/waitingfor` responses
    pub color: String,
    /// Player's display name
    pub name: String,
    /// URL to the player's game page
    ///
    /// Direct link to access this player's view of the game on the Terraforming Mars server
    pub url: String,
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "id={}, name= {}, color={}",
            self.id, self.name, self.color
        )
    }
}

/// Different phases of a Terraforming Mars game.
///
/// Based on the official implementation:
/// <https://github.com/terraforming-mars/terraforming-mars/blob/main/src/common/Phase.ts>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// Not part of the rulebook, initial drafting includes project cards and
    /// prelude cards (maybe others ongoing?) Transitions to RESEARCH
    /// but as mentioned above, only the first generation type of research.
    InitialDrafting,
    /// Between 1st gen research and action phases, each player plays their preludes.
    Preludes,
    /// Between 1st gen research and action phases, each player plays their CEOs.
    Ceos,
    /// The phase where a player chooses cards to keep.
    /// This includes the first generation drafting phase, which has different
    /// behavior and transitions to a different eventual phase
    Research,
    /// The standard drafting phase, as described by the official rules variant.
    Drafting,
    /// Maps to rulebook action phase
    Action,
    /// Maps to rulebook production phase */
    Production,
    /// Standard rulebook Solar phase, triggers WGT, and final greeneries, but not Turmoil.
    Solar,
    /// Does some cleanup and also executes the rulebook's turn order phase.
    Intergeneration,
    /// The game is over.
    End,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_display() {
        let game = Game {
            id: "game789".to_string(),
            phase: Phase::Production,
            spectator_id: "spec999".to_string(),
            players: vec![],
            waited_players: HashSet::new(),
        };

        let display = format!("{}", game);
        assert!(display.contains("id=game789"));
        assert!(display.contains("phase=Production"));
        assert!(display.contains("spectator_id=spec999"));
    }

    #[test]
    fn test_player_display() {
        let player = Player {
            id: "p456".to_string(),
            color: "yellow".to_string(),
            name: "Diana".to_string(),
            url: "https://example.com/player/p456".to_string(),
        };

        let display = format!("{}", player);
        assert!(display.contains("id=p456"));
        assert!(display.contains("name= Diana"));
        assert!(display.contains("color=yellow"));
    }
}
