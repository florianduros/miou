//! HTTP client for Terraforming Mars server API.
//!
//! This module provides the [`TMarsRequester`] struct for making HTTP requests
//! to the Terraforming Mars server and retrieving game information.

use log::{debug, info};
use mockall::automock;
use reqwest::{Client, Error};

use crate::tmars::response_structs::{GameDetail, GameResponse, WaitingForResponse};

/// HTTP client for requesting data from the Terraforming Mars server.
///
/// # Examples
///
/// ```no_run
/// let mars_requester = TMarsRequester::new("your_server_id".to_string(), "http://your_tmars_server_url".to_string());
/// let games = mars_requester.get_games().await.unwrap();
/// println!("Games: {:?}", games);
/// ```
pub struct TMarsRequester {
    /// Terraforming mars secret server id
    ///
    /// The server id is displayed at the server start up
    server_id: String,
    /// Terraforming mars server url
    url: String,
    /// HTTP client
    client: Client,
}

/// Trait for making requests to the TMars server.
///
/// This trait abstracts the HTTP operations for easier testing with mocks.
#[automock]
pub trait Requester {
    /// Fetches the list of active games.
    async fn get_games(&self) -> Result<Vec<GameResponse>, Error>;
    /// Fetches detailed information about a specific game.
    async fn get_game_details(&self, game_id: &str) -> Result<GameDetail, Error>;
    /// Fetches the list of players being waited for in a game.
    async fn get_waited_players(&self, player_id: &str) -> Result<WaitingForResponse, Error>;
    fn get_player_url(&self, player_id: &str) -> String;
}

impl TMarsRequester {
    /// Create a new [TMarsRequester].
    ///
    /// # Arguments
    ///
    /// * `url` - The base URL of the Terraforming Mars server.
    /// * `server_id` - The secret server ID for authentication.
    pub fn new(url: &str, server_id: &str) -> Self {
        let client = reqwest::Client::new();
        TMarsRequester {
            server_id: server_id.to_string(),
            url: url.to_string(),
            client,
        }
    }
}

impl Requester for TMarsRequester {
    /// Request `/api/games` to get the list of games.
    ///
    /// This api call returns a json array of games:
    /// ```
    /// [
    ///   { gameId: "randomIdA", participantIds: ["partId1", "partId2"] },
    ///   { gameId: "randomIdB", participantIds: ["partId3", "partId4", "partId5"] }
    /// ]
    /// ```
    /// This method transforms this json into a [`GameResponse`] vector.
    /// ParticipantIds are ignored because [`Self::get_game_details`] includes them.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let games = tmars_requester.get_games().await.unwrap();
    /// println!("Games: {:?}", games);
    /// ```
    async fn get_games(&self) -> Result<Vec<GameResponse>, Error> {
        let url = format!("{}/api/games", &self.url);
        info!("request games");
        debug!("request {}?serverId={}", &url, &self.server_id);

        let game_responses: Vec<GameResponse> = self
            .client
            .get(&url)
            .query(&[("serverId", &self.server_id)])
            .send()
            .await?
            .json()
            .await?;

        debug!("response from {} -> {:?}", &url, &game_responses);

        Ok(game_responses)
    }

    /// Request `/api/game?id={gameId}` to get the details of a specific game.
    ///
    /// This api call returns a json object representing the game details:
    /// ```
    /// {
    ///   id: "gameId",
    ///   spectatorId: "spectatorId",
    ///   players: [
    ///     { id: "playerId1", name: "Alice", color: "red" },
    ///     { id: "playerId2", name: "Bob", color: "green" }
    ///   ]
    /// }
    /// ```
    /// This method transforms this json into a [`GameDetail`].
    ///
    /// # Arguments
    ///
    /// * `game_id` - The unique identifier of the game to get details for.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let game_detail = tmars_requester.get_game_details("game_id").await.unwrap();
    /// println!("Game detail: {:?}", game_detail);
    /// ```
    async fn get_game_details(&self, game_id: &str) -> Result<GameDetail, Error> {
        let url = format!("{}/api/game", &self.url);
        info!("request game details of {}", &game_id);
        debug!("request {}?id={}", &url, &game_id);

        let game_detail: GameDetail = self
            .client
            .get(&url)
            .query(&[("id", game_id)])
            .send()
            .await?
            .json()
            .await?;

        debug!(
            "response from {}?id={} -> {:?}",
            &url, &game_id, &game_detail
        );

        Ok(game_detail)
    }

    /// Request `/api/waitingfor?id={spectactorId}` to get of waited players. The players are identified by their color instead of their id.
    ///
    /// This api call returns a json object representing the list of players the spectator is waiting for:
    /// ```
    /// {
    ///   waitingFor: ["green", "red"]
    /// }
    /// ```
    /// This method transforms this json into a [`WaitingForResponse`].
    ///
    /// # Arguments
    ///
    /// * `player_id` - The spectator id to get the waited players for.
    ///
    /// # Examples
    ////
    /// ```no_run
    /// let waiting_for = tmars_requester.get_waited_players("spectator_id").await.unwrap();
    /// println!("Waiting for players: {:?}", waiting_for);
    /// ```
    async fn get_waited_players(&self, player_id: &str) -> Result<WaitingForResponse, Error> {
        let url = format!("{}/api/waitingfor", &self.url);
        info!(
            "request list of waited players for spectator {}",
            &player_id
        );
        debug!("request {}?id={}", &url, &player_id);

        let waiting_for_response: WaitingForResponse = self
            .client
            .get(&url)
            .query(&[("id", player_id)])
            .send()
            .await?
            .json()
            .await?;

        debug!(
            "response from {}?id={} -> {:?}",
            &url, &player_id, &waiting_for_response
        );

        Ok(waiting_for_response)
    }

    fn get_player_url(&self, player_id: &str) -> String {
        format!("{}/player?id={}", &self.url, player_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_games() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let server_id = "abcd";
        let body = r#"[{"gameId": "game1"}, {"gameId": "game2"}]"#;

        server
            .mock("GET", "/api/games")
            .match_query(mockito::Matcher::UrlEncoded(
                "serverId".to_owned(),
                server_id.to_owned(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let tmars_requester = TMarsRequester::new(&url, server_id);
        let games = tmars_requester.get_games().await.unwrap();
        assert_eq!(games.len(), 2);
        assert_eq!(games.first().unwrap().game_id, "game1");
        assert_eq!(games.last().unwrap().game_id, "game2");
    }

    #[tokio::test]
    async fn test_get_game_details() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let game_id = "game1";
        let body = r#"{"id": "game1", "phase": "research", "spectatorId": "specId", "players": [{"id": "playerId1", "color": "green", "name": "Alice"}, {"id": "playerId2", "color": "red", "name": "Bob"}]}"#;

        server
            .mock("GET", "/api/game")
            .match_query(mockito::Matcher::UrlEncoded(
                "id".to_owned(),
                game_id.to_owned(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let tmars_requester = TMarsRequester::new(&url, "server_id");
        let game = tmars_requester.get_game_details(game_id).await.unwrap();
        assert_eq!(game.id, game_id);
        assert_eq!(game.spectator_id, "specId");

        assert_eq!(game.players.len(), 2);
        assert_eq!(game.players[0].id, "playerId1");
        assert_eq!(game.players[0].name, "Alice");
        assert_eq!(game.players[0].color, "green");
        assert_eq!(game.players[1].id, "playerId2");
        assert_eq!(game.players[1].name, "Bob");
        assert_eq!(game.players[1].color, "red");
    }

    #[tokio::test]
    async fn test_get_waited_players() {
        let mut server = mockito::Server::new_async().await;
        let url = server.url();
        let spectator_id = "specId";
        let body = r#"{"waitingFor": ["green", "red"]}"#;

        server
            .mock("GET", "/api/waitingfor")
            .match_query(mockito::Matcher::UrlEncoded(
                "id".to_owned(),
                spectator_id.to_owned(),
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create_async()
            .await;

        let tmars_requester = TMarsRequester::new(&url, "server_id");
        let waiting_for_response = tmars_requester
            .get_waited_players(spectator_id)
            .await
            .unwrap();
        assert_eq!(waiting_for_response.waiting_for.len(), 2);
        assert_eq!(waiting_for_response.waiting_for.first().unwrap(), "green");
        assert_eq!(waiting_for_response.waiting_for.last().unwrap(), "red");
    }

    #[test]
    fn test_get_player_url() {
        let tmars_requester = TMarsRequester::new("http://tmars.server", "server_id");
        assert_eq!(
            tmars_requester.get_player_url("123"),
            "http://tmars.server/player?id=123"
        )
    }
}
