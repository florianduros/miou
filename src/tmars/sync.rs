//! Game synchronization logic for TMars server.
//!
//! This module provides the [`TMarsSync`] struct that manages periodic
//! synchronization with the TMars server to keep game state up to date.

use crate::tmars::requester::Requester;
use crate::tmars::response_structs::{GameDetail, PlayerDetail};
use crate::tmars::structs::{Game, Phase, Player};
use futures::future::join_all;

use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};

/// Manages synchronization of game state with the Terraforming Mars server.
///
/// This module defines the [TMarsSync] struct, which is responsible for synchronizing game details.
/// It interacts with a tmars server through a [Requester] implementation to fetch game information.
/// It maintains an internal state of synchronized games and their players.
///
/// # Examples
///
/// ```no_run
/// use miou::tmars::{TMarsSync, TMarsRequester};
///
/// # #[tokio::main]
/// # async fn main() {
/// let tmars_requester = TMarsRequester::new("your_server_id".to_string(), "http://your_tmars_server_url".to_string());
/// let mut tmars_sync = TMarsSync::new(tmars_requester);
/// tmars_sync.sync().await;
/// # }
/// ```
pub struct TMarsSync<R: Requester> {
    /// TMars requester to interact with the tmars server
    tmars_requester: R,
    /// Synchronized games
    games: HashMap<String, Game>,
}

impl<R: Requester> TMarsSync<R> {
    /// Create a new [TMarsSync].
    ///
    /// # Arguments
    ///
    /// * `tmars_requester` - An implementation of the [Requester] trait to interact with the tmars server.
    pub fn new(tmars_requester: R) -> Self {
        let games = HashMap::new();

        TMarsSync {
            tmars_requester,
            games,
        }
    }

    /// Synchronizes game state by fetching game details and waiting players.
    ///
    /// This method performs a full synchronization cycle by:
    /// 1. Fetching all active games and their details
    /// 2. Updating the list of players being waited for in each game
    ///
    /// This should be called periodically to keep the internal state current.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::tmars::{TMarsSync, TMarsRequester};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let tmars_requester = TMarsRequester::new("server_id".to_string(), "http://server_url".to_string());
    /// let mut tmars_sync = TMarsSync::new(tmars_requester);
    /// tmars_sync.sync().await;
    /// # }
    /// ```
    pub async fn sync(&mut self) {
        self.pool_games().await;
        self.pool_waited_players().await;
    }

    /// Fetches all games from the server and updates internal state.
    ///
    /// This method:
    /// - Requests the list of all active games
    /// - Fetches detailed information for each game
    /// - Filters out ended games
    /// - Updates the internal `games` HashMap with current data
    ///
    /// The previous game state is completely replaced with the new data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::tmars::{TMarsSync, TMarsRequester};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let tmars_requester = TMarsRequester::new("server_id".to_string(), "http://server_url".to_string());
    /// let mut tmars_sync = TMarsSync::new(tmars_requester);
    /// tmars_sync.pool_games().await;
    /// # }
    /// ```
    async fn pool_games(&mut self) {
        info!("request games from tmars server");
        let game_details = self.request_games().await;

        // Clear existing games to avoid stale data
        self.games.clear();

        game_details.into_iter().for_each(|game_detail| {
            debug!("sync game detail {}", game_detail);

            // Ignore ended games
            if game_detail.phase == "end" {
                debug!("ignore game {}, phase=end", game_detail.id);
                return;
            }

            // Convert the deserialized players into internal Player structs
            let players = self.convert_players(game_detail.players);
            // Convert phase string to Phase enum
            let phase = self.convert_phase(&game_detail.phase);

            let game = Game {
                id: game_detail.id.to_owned(),
                phase,
                spectator_id: game_detail.spectator_id.to_owned(),
                players,
                waited_players: HashSet::new(),
            };

            if !self.games.contains_key(&game.id) {
                info!("add new game {}", game);
            }

            debug!("synced game {}", game);
            self.games.insert(game.id.to_owned(), game);
        });

        debug!("all games {:?}", self.games);
        info!("finished requesting games from tmars server");
    }

    /// Requests the list of games and fetches details for each.
    ///
    /// Makes parallel requests for game details to improve performance.
    /// Errors for individual games are logged and filtered out.
    ///
    /// # Returns
    ///
    /// A vector of successfully fetched [`GameDetail`] objects.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::tmars::{TMarsSync, TMarsRequester};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let tmars_requester = TMarsRequester::new("server_id".to_string(), "http://server_url".to_string());
    /// let tmars_sync = TMarsSync::new(tmars_requester);
    /// let game_details = tmars_sync.request_games().await;
    /// println!("Game details: {:?}", game_details);
    /// # }
    /// ```
    async fn request_games(&self) -> Vec<GameDetail> {
        // Request list of games
        let game_ids = match self.tmars_requester.get_games().await {
            Ok(games) => games,
            Err(e) => {
                error!("error while requesting games: {}", e);
                vec![]
            }
        };

        // Request details for each game
        let game_details = join_all(
            game_ids
                .iter()
                .map(|g| self.tmars_requester.get_game_details(&g.game_id)),
        )
        .await;

        // Filter out errors and collect valid game details
        game_details
            .into_iter()
            // Keep only successful game detail responses
            .filter_map(|g| match g {
                Ok(game_detail) => Some(game_detail),
                Err(err) => {
                    error!("error while getting game details: {}", err);
                    None
                }
            })
            .collect()
    }

    /// Converts a phase string from the API into a [`Phase`] enum.
    ///
    /// # Arguments
    ///
    /// * `phase` - A string slice representing the phase (camelCase format).
    ///
    /// # Returns
    ///
    /// The corresponding [`Phase`] enum variant, or [`Phase::Research`] if unknown.
    ///
    /// # Examples
    ///
    /// ```
    /// use miou::tmars::TMarsSync;
    /// use miou::tmars::requester::MockRequester;
    /// use miou::tmars::structs::Phase;
    ///
    /// # fn main() {
    /// let mock_requester = MockRequester::new();
    /// let tmars_sync = TMarsSync::new(mock_requester);
    /// let phase = tmars_sync.convert_phase("research");
    /// assert!(matches!(phase, Phase::Research));
    /// # }
    /// ```
    fn convert_phase(&self, phase: &str) -> Phase {
        match phase {
            "initialDrafting" => Phase::InitialDrafting,
            "preludes" => Phase::Preludes,
            "ceos" => Phase::Ceos,
            "research" => Phase::Research,
            "drafting" => Phase::Drafting,
            "action" => Phase::Action,
            "production" => Phase::Production,
            "solar" => Phase::Solar,
            "intergeneration" => Phase::Intergeneration,
            "end" => Phase::End,
            _ => {
                warn!("unknown phase string: {}", phase);
                Phase::Research // Default to Research if unknown
            }
        }
    }

    /// Converts API player details into internal player representations.
    ///
    /// Transforms [`PlayerDetail`] (from API responses) into [`Player`]
    /// (internal representation) by copying relevant fields.
    ///
    /// # Arguments
    ///
    /// * `players` - Vector of [`PlayerDetail`] from the API response.
    ///
    /// # Returns
    ///
    /// Vector of [`Player`] structs for internal use.
    ///
    /// # Examples
    ///
    /// ```
    /// use miou::tmars::TMarsSync;
    /// use miou::tmars::requester::MockRequester;
    /// use miou::tmars::response_structs::PlayerDetail;
    ///
    /// # fn main() {
    /// let mock_requester = MockRequester::new();
    /// let tmars_sync = TMarsSync::new(mock_requester);
    /// let player_details = vec![PlayerDetail {
    ///     id: "1".to_string(),
    ///     name: "Alice".to_string(),
    ///     color: "red".to_string()
    /// }];
    /// let players = tmars_sync.convert_players(player_details);
    /// assert_eq!(players.len(), 1);
    /// assert_eq!(players[0].name, "Alice");
    /// # }
    /// ```
    fn convert_players(&self, players: Vec<PlayerDetail>) -> Vec<Player> {
        players
            .iter()
            .map(|player_detail| Player {
                id: player_detail.id.to_owned(),
                color: player_detail.color.to_owned(),
                name: player_detail.name.to_owned(),
                url: self.tmars_requester.get_player_url(&player_detail.id),
            })
            .collect()
    }

    /// Fetches and updates the list of players being waited for in all games.
    ///
    /// For each synchronized game, this method:
    /// 1. Requests the waiting players from the server using the spectator ID
    /// 2. Converts player colors to player IDs
    /// 3. Updates the game's `waited_players` field
    ///
    /// Requests are made in parallel for better performance.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::tmars::{TMarsSync, TMarsRequester};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let tmars_requester = TMarsRequester::new("server_id".to_string(), "http://server_url".to_string());
    /// let mut tmars_sync = TMarsSync::new(tmars_requester);
    /// tmars_sync.pool_waited_players().await;
    /// # }
    /// ```
    async fn pool_waited_players(&mut self) {
        info!("request waited players from tmars server");

        // Collect game info first to avoid borrow checker issues
        let game_info: Vec<(String, String)> = self
            .games
            .values()
            .map(|game| (game.id.clone(), game.spectator_id.clone()))
            .collect();

        // Request waited players for all games
        let waited_players_futures = game_info
            .iter()
            .map(|(_, spectator_id)| self.request_waited_players(spectator_id));
        let waited_players_results = join_all(waited_players_futures).await;

        // Update games with waited players
        for ((game_id, _), waited_players_colors) in game_info.iter().zip(waited_players_results) {
            if let Some(game) = self.games.get_mut(game_id) {
                if waited_players_colors.is_empty() {
                    debug!("no players are being waited for in game {}", game.id);
                    game.waited_players.clear();
                    continue;
                }

                debug!(
                    "game {}: waiting for players with colors {:?}",
                    game.id, waited_players_colors
                );

                // Map colors to player IDs
                let waited_players: Vec<String> = game
                    .players
                    .iter()
                    .filter_map(|player| {
                        if waited_players_colors.contains(&player.color) {
                            Some(player.id.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect();

                debug!(
                    "game {}: mapped to player IDs {:?}",
                    game.id, waited_players
                );

                game.waited_players = waited_players.into_iter().collect();
            }
        }

        info!("finished requesting waited players from tmars server");
    }

    /// Requests the list of player colors being waited for.
    ///
    /// # Arguments
    ///
    /// * `spectator_id` - The spectator ID for the game.
    ///
    /// # Returns
    ///
    /// Vector of player colors (strings) being waited for, or empty vector on error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::tmars::{TMarsSync, TMarsRequester};
    ///
    /// # #[tokio::main]
    /// # async fn main() {
    /// let tmars_requester = TMarsRequester::new("server_id".to_string(), "http://server_url".to_string());
    /// let tmars_sync = TMarsSync::new(tmars_requester);
    /// let waited_players = tmars_sync.request_waited_players("spectator_id").await;
    /// println!("Waited players: {:?}", waited_players);
    /// # }
    /// ```
    async fn request_waited_players(&self, spectator_id: &str) -> Vec<String> {
        match self.tmars_requester.get_waited_players(spectator_id).await {
            Ok(waited_players) => waited_players.waiting_for,
            Err(e) => {
                debug!(
                    "error while requesting waited players {}: {}",
                    spectator_id, e
                );
                vec![]
            }
        }
    }

    /// Returns a clone of all synchronized games.
    ///
    /// # Returns
    ///
    /// A [`HashMap`] mapping game IDs to their corresponding [`Game`] objects.
    ///
    /// # Examples
    ///
    /// ```
    /// use miou::tmars::TMarsSync;
    /// use miou::tmars::requester::MockRequester;
    ///
    /// # fn main() {
    /// let mock_requester = MockRequester::new();
    /// let tmars_sync = TMarsSync::new(mock_requester);
    /// let games = tmars_sync.get_games();
    /// assert_eq!(games.len(), 0); // No games synchronized yet
    /// # }
    /// ```
    pub fn get_games(&self) -> HashMap<String, Game> {
        self.games.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmars::requester::MockRequester;
    use crate::tmars::response_structs::{GameResponse, WaitingForResponse};

    #[tokio::test]
    async fn test_pool_games() {
        let mut mock_requester = MockRequester::new();

        // Mock get_player_url calls for all players
        mock_requester
            .expect_get_player_url()
            .with(mockall::predicate::eq("player1"))
            .times(1)
            .returning(|id| format!("http://example.com/{}", id));

        mock_requester
            .expect_get_player_url()
            .with(mockall::predicate::eq("player2"))
            .times(1)
            .returning(|id| format!("http://example.com/{}", id));

        mock_requester
            .expect_get_player_url()
            .with(mockall::predicate::eq("player3"))
            .times(1)
            .returning(|id| format!("http://example.com/{}", id));

        // Mock get_games response
        mock_requester.expect_get_games().times(1).returning(|| {
            Ok(vec![
                GameResponse {
                    game_id: "game1".to_owned(),
                },
                GameResponse {
                    game_id: "game2".to_owned(),
                },
            ])
        });

        // Mock get_game_details for game1
        mock_requester
            .expect_get_game_details()
            .with(mockall::predicate::eq("game1"))
            .times(1)
            .returning(|_| {
                Ok(GameDetail {
                    id: "game1".to_owned(),
                    phase: "research".to_owned(),
                    spectator_id: "spec1".to_owned(),
                    players: vec![
                        PlayerDetail {
                            id: "player1".to_owned(),
                            name: "Alice".to_owned(),
                            color: "red".to_owned(),
                        },
                        PlayerDetail {
                            id: "player2".to_owned(),
                            name: "Bob".to_owned(),
                            color: "blue".to_owned(),
                        },
                    ],
                })
            });

        // Mock get_game_details for game2
        mock_requester
            .expect_get_game_details()
            .with(mockall::predicate::eq("game2"))
            .times(1)
            .returning(|_| {
                Ok(GameDetail {
                    id: "game2".to_owned(),
                    phase: "action".to_owned(),
                    spectator_id: "spec2".to_owned(),
                    players: vec![PlayerDetail {
                        id: "player3".to_owned(),
                        name: "Charlie".to_owned(),
                        color: "green".to_owned(),
                    }],
                })
            });

        let mut tmars_sync = TMarsSync::new(mock_requester);

        // Test pool_games method
        tmars_sync.pool_games().await;

        // Verify that games have been populated correctly
        assert_eq!(tmars_sync.games.len(), 2);

        let game1 = tmars_sync.games.get("game1").unwrap();
        assert_eq!(game1.id, "game1");
        assert_eq!(game1.spectator_id, "spec1");
        assert_eq!(game1.players.len(), 2);
        assert_eq!(game1.players[0].name, "Alice");
        assert_eq!(game1.players[1].name, "Bob");

        let game2 = tmars_sync.games.get("game2").unwrap();
        assert_eq!(game2.id, "game2");
        assert_eq!(game2.spectator_id, "spec2");
        assert_eq!(game2.players.len(), 1);
        assert_eq!(game2.players[0].name, "Charlie");
    }

    #[tokio::test]
    async fn test_pool_waited_players() {
        let mut mock_requester = MockRequester::new();

        // Setup mock for get_waited_players
        mock_requester
            .expect_get_waited_players()
            .with(mockall::predicate::eq("spec1"))
            .times(1)
            .returning(|_| {
                Ok(WaitingForResponse {
                    waiting_for: vec!["red".to_owned(), "blue".to_owned()],
                })
            });

        let mut tmars_sync = TMarsSync::new(mock_requester);

        // Manually add a game to test waited players functionality
        let game = Game {
            id: "game1".to_owned(),
            phase: Phase::Research,
            spectator_id: "spec1".to_owned(),
            players: vec![
                Player {
                    id: "player1".to_owned(),
                    name: "Alice".to_owned(),
                    color: "red".to_owned(),
                    url: "http://example.com/player1".to_owned(),
                },
                Player {
                    id: "player2".to_owned(),
                    name: "Bob".to_owned(),
                    color: "blue".to_owned(),
                    url: "http://example.com/player2".to_owned(),
                },
                Player {
                    id: "player3".to_owned(),
                    name: "Charlie".to_owned(),
                    color: "green".to_owned(),
                    url: "http://example.com/player3".to_owned(),
                },
            ],
            waited_players: HashSet::new(),
        };

        tmars_sync.games.insert("game1".to_owned(), game);

        // Test pool_waited_players method
        tmars_sync.pool_waited_players().await;

        // Verify waited players were set correctly
        let game = tmars_sync.games.get("game1").unwrap();
        assert_eq!(game.waited_players.len(), 2);
        assert!(game.waited_players.contains("player1"));
        assert!(game.waited_players.contains("player2"));
        assert!(!game.waited_players.contains("player3"));
    }

    #[test]
    fn test_convert_phase() {
        let mock_requester = MockRequester::new();
        let tmars_sync = TMarsSync::new(mock_requester);

        assert!(matches!(
            tmars_sync.convert_phase("research"),
            Phase::Research
        ));
        assert!(matches!(tmars_sync.convert_phase("action"), Phase::Action));
        assert!(matches!(
            tmars_sync.convert_phase("production"),
            Phase::Production
        ));
        assert!(matches!(tmars_sync.convert_phase("end"), Phase::End));
        assert!(matches!(
            tmars_sync.convert_phase("unknown"),
            Phase::Research
        )); // Default
    }

    #[test]
    fn test_convert_players() {
        let mut mock_requester = MockRequester::new();

        // Mock get_player_url calls
        mock_requester
            .expect_get_player_url()
            .with(mockall::predicate::eq("player1"))
            .times(1)
            .returning(|id| format!("http://example.com/{}", id));

        mock_requester
            .expect_get_player_url()
            .with(mockall::predicate::eq("player2"))
            .times(1)
            .returning(|id| format!("http://example.com/{}", id));

        let tmars_sync = TMarsSync::new(mock_requester);

        let player_details = vec![
            PlayerDetail {
                id: "player1".to_owned(),
                name: "Alice".to_owned(),
                color: "red".to_owned(),
            },
            PlayerDetail {
                id: "player2".to_owned(),
                name: "Bob".to_owned(),
                color: "blue".to_owned(),
            },
        ];

        let players = tmars_sync.convert_players(player_details);

        assert_eq!(players.len(), 2);
        assert_eq!(players[0].id, "player1");
        assert_eq!(players[0].name, "Alice");
        assert_eq!(players[0].color, "red");
        assert_eq!(players[0].url, "http://example.com/player1");
        assert_eq!(players[1].id, "player2");
        assert_eq!(players[1].name, "Bob");
        assert_eq!(players[1].color, "blue");
        assert_eq!(players[1].url, "http://example.com/player2");
    }

    #[tokio::test]
    async fn test_get_games() {
        let mock_requester = MockRequester::new();
        let mut tmars_sync = TMarsSync::new(mock_requester);

        // Manually add a game to test
        let game = Game {
            id: "game1".to_owned(),
            phase: Phase::Research,
            spectator_id: "spec1".to_owned(),
            players: vec![
                Player {
                    id: "player1".to_owned(),
                    name: "Alice".to_owned(),
                    color: "red".to_owned(),
                    url: "http://example.com/player1".to_owned(),
                },
                Player {
                    id: "player2".to_owned(),
                    name: "Bob".to_owned(),
                    color: "blue".to_owned(),
                    url: "http://example.com/player2".to_owned(),
                },
                Player {
                    id: "player3".to_owned(),
                    name: "Charlie".to_owned(),
                    color: "green".to_owned(),
                    url: "http://example.com/player3".to_owned(),
                },
            ],
            waited_players: HashSet::new(),
        };

        tmars_sync.games.insert("game1".to_owned(), game.clone());

        // game1 should be returned
        let games_map = tmars_sync.get_games();
        assert_eq!(games_map.get("game1").unwrap(), &game);
    }
}
