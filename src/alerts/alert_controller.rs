//! Alert controller for managing game notifications and their lifecycle.
//!
//! This module provides the [`AlertController`] which orchestrates alert management,
//! notification scheduling, and persistence for Terraforming Mars game alerts.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};

use log::{debug, info};
use tokio::{sync::Mutex, task::JoinHandle, time};

use crate::{
    alerts::{Alert, AlertLoader},
    tmars::Game,
};

type GamesMap = HashMap<String, Game>;

/// Interval in seconds between automatic alert persistence operations.
const SAVE_INTERVAL_SECS: u64 = 60; // 1 minute

/// Manages the lifecycle of alerts for Terraforming Mars games.
///
/// The `AlertController` coordinates several key responsibilities:
/// - Tracking active alerts across multiple games
/// - Scheduling delayed notifications when a player's turn arrives
/// - Cleaning up alerts for games that no longer exist
/// - Persisting alerts to disk periodically
/// - Managing background tasks for notifications
///
/// # Thread Safety
///
/// All public methods are async and use internal locking to ensure thread-safe
/// concurrent access to the alerts map and notification tasks.
///
/// # Examples
///
/// ```no_run
/// use miou::alerts::AlertController;
///
/// # async fn example() {
/// // Initialize the controller
/// let mut controller = AlertController::new("alerts.json".to_string()).await;
///
/// // Start automatic persistence
/// controller.start_persistence_task();
///
/// // The controller is now ready to manage alerts
/// # }
/// ```
pub struct AlertController {
    /// Thread-safe reference to the alerts map
    alerts_map: Arc<Mutex<HashMap<String, HashSet<Alert>>>>,
    /// Loader for persisting and loading alerts from disk
    alert_loader: AlertLoader,
    /// Map of active alerts to their notification task handles
    thread_handles_map: HashMap<Alert, JoinHandle<()>>,
}

impl AlertController {
    /// Creates a new `AlertController` and loads existing alerts from disk.
    ///
    /// # Arguments
    ///
    /// * `alerts_path` - Path to the JSON file where alerts are persisted
    ///
    /// # Returns
    ///
    /// A new `AlertController` instance with alerts loaded from the specified file.
    /// If the file doesn't exist or is corrupted, starts with an empty alerts map.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::alerts::AlertController;
    ///
    /// # async fn example() {
    /// let controller = AlertController::new("alerts.json".to_string()).await;
    /// # }
    /// ```
    pub async fn new(alerts_path: String) -> Self {
        let alert_loader = AlertLoader::new(alerts_path);
        let alerts_map = Arc::new(Mutex::new(alert_loader.load().await));
        let thread_handles_map: HashMap<Alert, JoinHandle<()>> = HashMap::new();

        AlertController {
            alerts_map,
            alert_loader,
            thread_handles_map,
        }
    }

    /// Returns a clone of the current alerts map.
    ///
    /// # Returns
    ///
    /// A `HashMap` mapping game IDs to sets of alerts for each game.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::alerts::AlertController;
    ///
    /// # async fn example() {
    /// let controller = AlertController::new("alerts.json".to_string()).await;
    /// let alerts = controller.get_alerts_map().await;
    /// println!("Managing alerts for {} games", alerts.len());
    /// # }
    /// ```
    pub async fn get_alerts_map(&self) -> HashMap<String, HashSet<Alert>> {
        self.alerts_map.lock().await.clone()
    }

    /// Starts a background task that periodically persists alerts to disk.
    ///
    /// The task runs indefinitely, saving the current alerts map to disk every
    /// [`SAVE_INTERVAL_SECS`] seconds. This ensures alerts are not lost if the
    /// bot crashes or restarts.
    ///
    /// # Note
    ///
    /// This method spawns a background Tokio task and returns immediately.
    /// The task continues running until the program exits.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::alerts::AlertController;
    ///
    /// # async fn example() {
    /// let controller = AlertController::new("alerts.json".to_string()).await;
    /// controller.start_persistence_task();
    /// // Alerts will now be automatically saved every minute
    /// # }
    /// ```
    pub fn start_persistence_task(&self) {
        let alerts_map = Arc::clone(&self.alerts_map);
        let alert_loader = self.alert_loader.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(SAVE_INTERVAL_SECS));
            loop {
                interval.tick().await;
                let alerts = alerts_map.lock().await;
                alert_loader.persist_alerts_map(&alerts).await;
            }
        });
    }

    /// Updates alerts based on current game state and triggers notifications.
    ///
    /// This method performs three key operations:
    /// 1. Removes alerts for games that no longer exist
    /// 2. Identifies alerts that should fire based on current game state
    /// 3. Schedules delayed notifications for those alerts
    ///
    /// # Arguments
    ///
    /// * `games_map` - Current state of all active games
    /// * `on_alert_to_fire` - Callback function invoked when an alert fires
    ///
    /// # Type Parameters
    ///
    /// * `F` - A function that takes an `Alert` and performs the notification action
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::alerts::{Alert, AlertController};
    /// use std::collections::HashMap;
    ///
    /// # async fn example() {
    /// let mut controller = AlertController::new("alerts.json".to_string()).await;
    /// let games_map = HashMap::new(); // Normally populated with actual games
    ///
    /// controller.update_alerts(&games_map, |alert| {
    ///     println!("Alert fired for user {}", alert.user_id);
    /// }).await;
    /// # }
    /// ```
    pub async fn update_alerts<F>(&mut self, games_map: &GamesMap, on_alert_to_fire: F)
    where
        F: Fn(Alert) + Send + Sync + 'static,
    {
        self.clean_alerts(games_map).await;
        let alerts_to_fire = self.get_alerts_to_fire(games_map).await;

        let on_alert_to_fire = Arc::new(on_alert_to_fire);
        self.fire_alert(alerts_to_fire, move |alert| {
            let on_alert_to_fire = Arc::clone(&on_alert_to_fire);
            on_alert_to_fire(alert);
        });
    }

    /// Removes alerts for games that no longer exist.
    ///
    /// Iterates through all alerts and removes entries for game IDs that are not
    /// present in the current games map. Also aborts any pending notification tasks
    /// for those alerts.
    ///
    /// # Arguments
    ///
    /// * `games_map` - Current state of all active games
    ///
    /// # Side Effects
    ///
    /// - Removes alerts from the alerts map
    /// - Aborts notification tasks for removed alerts
    /// - Logs info messages for each cleaned game
    async fn clean_alerts(&mut self, games_map: &GamesMap) {
        let thread_handles_map = &mut self.thread_handles_map;

        self.alerts_map.lock().await.retain(|game_id, alerts| {
            if games_map.contains_key(game_id) {
                true
            } else {
                // Remove all thread handles associated with these alerts
                alerts.iter().for_each(|alert| {
                    if let Some((_, handle)) = thread_handles_map.remove_entry(alert) {
                        handle.abort();
                    }
                });
                info!("removing alerts for non-existing game {}", game_id);
                false
            }
        });
    }

    /// Identifies alerts that should trigger notifications based on game state.
    ///
    /// Compares the current game state against registered alerts to determine:
    /// - Which alerts need to fire (player's turn arrived, not yet notified)
    /// - Which alerts need their notified status reset (player's turn ended)
    ///
    /// # Arguments
    ///
    /// * `games_map` - Current state of all active games
    ///
    /// # Returns
    ///
    /// A vector of tuples containing (game_id, alert) for each alert that should fire.
    ///
    /// # Behavior
    ///
    /// For each game with registered alerts:
    /// - If a player's turn has arrived and they haven't been notified: mark for firing
    /// - If a player's turn has ended and they were notified: reset the notified flag
    /// - Abort notification tasks for alerts that are no longer needed
    async fn get_alerts_to_fire(&mut self, games_map: &GamesMap) -> Vec<(String, Alert)> {
        let mut alerts_map = self.alerts_map.lock().await;

        // Collection of alerts that should trigger notifications
        let mut alerts_to_fire: Vec<(String, Alert)> = Vec::new();

        for (game_id, game) in games_map {
            // Only process games that have registered alerts
            let Some(alerts) = alerts_map.get_mut(game_id) else {
                continue;
            };
            // Temporary storage for alerts that need their notified status updated
            // Tuple contains (alert, new_notified_status)
            let mut alerts_to_update: Vec<(Alert, bool)> = Vec::new();

            // Check each alert against the current game state
            for alert in alerts.iter() {
                // If it's the player's turn and they haven't been notified yet
                if game.waited_players.contains(&alert.player_id) && !alert.notified {
                    alerts_to_fire.push((game_id.clone(), alert.clone()));
                    // Mark this alert for updating to prevent duplicate notifications
                    alerts_to_update.push((alert.clone(), true));
                } else if !game.waited_players.contains(&alert.player_id) && alert.notified {
                    // Reset the notified flag when it's no longer the player's turn
                    // This allows re-notification on their next turn
                    alerts_to_update.push((alert.clone(), false));
                    // Also abort any existing notification thread since it's no longer needed
                    if let Some((_, handle)) = self.thread_handles_map.remove_entry(alert) {
                        handle.abort();
                    }
                }
            }

            // Update alerts in the HashSet by removing and re-inserting
            // This is necessary because HashSet doesn't support in-place mutation
            for (mut alert, notified) in alerts_to_update {
                alerts.remove(&alert);
                alert.notified = notified;
                alerts.insert(alert);
            }
        }

        debug!("{:?} alerts to fire", alerts_to_fire);

        alerts_to_fire
    }

    /// Spawns delayed notification tasks for the given alerts.
    ///
    /// For each alert, this method:
    /// 1. Aborts any existing notification task for the same alert
    /// 2. Spawns a new async task that waits for the alert's delay period
    /// 3. Calls the callback function when the delay expires
    /// 4. Stores the task handle for potential future cancellation
    ///
    /// # Arguments
    ///
    /// * `alerts_to_fire` - Vector of (game_id, alert) tuples to schedule
    /// * `on_alert_to_fire` - Callback function invoked after the delay
    ///
    /// # Note
    ///
    /// Tasks can be aborted if the player's turn ends before the delay expires.
    fn fire_alert(
        &mut self,
        alerts_to_fire: Vec<(String, Alert)>,
        on_alert_to_fire: impl Fn(Alert) + Send + Sync + 'static,
    ) {
        let on_alert_to_fire = Arc::new(on_alert_to_fire);

        for (game_id, alert) in alerts_to_fire {
            // Abort existing thread if any
            if let Some(handle) = self.thread_handles_map.get(&alert) {
                handle.abort();
            }

            let on_alert_to_fire = Arc::clone(&on_alert_to_fire);
            let alert_clone = alert.clone();
            let handle = tokio::spawn(async move {
                debug!(
                    "waiting {} minutes before notifying user {} for game {}",
                    alert_clone.delay, alert_clone.user_id, game_id
                );

                // Wait for the specified delay before sending the notification
                time::sleep(Duration::from_secs(alert_clone.delay * 60)).await;

                info!(
                    "notifying user {} in room {} for game {}",
                    alert_clone.user_id, alert_clone.room_id, game_id
                );

                on_alert_to_fire(alert_clone);
            });
            self.thread_handles_map.insert(alert, handle);
        }
    }

    /// Adds a new alert to the alerts map.
    ///
    /// Registers an alert for a specific game. If this is the first alert for the game,
    /// creates a new entry in the map. If an identical alert already exists (same room,
    /// player, and user), it will be replaced due to HashSet semantics.
    ///
    /// # Arguments
    ///
    /// * `game_id` - The ID of the game to register the alert for
    /// * `alert` - Reference to the alert to register
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::alerts::{Alert, AlertController};
    ///
    /// # async fn example() {
    /// let controller = AlertController::new("alerts.json".to_string()).await;
    ///
    /// let alert = Alert {
    ///     room_id: "!room:example.com".to_string(),
    ///     player_id: "player123".to_string(),
    ///     user_id: "@user:example.com".to_string(),
    ///     notified: false,
    ///     delay: 60,
    ///     player_url: "https://example.com/player".to_string(),
    /// };
    ///
    /// controller.add_alert("game_id", &alert).await;
    /// # }
    /// ```
    pub async fn add_alert(&self, game_id: &str, alert: &Alert) {
        let mut alerts_map = self.alerts_map.lock().await;

        let alerts = alerts_map
            .entry(game_id.to_owned())
            .or_insert_with(HashSet::new);

        alerts.insert(alert.to_owned());

        info!(
            "registered alert for player {} for game {} for user {} with delay {} minutes",
            alert.player_id, game_id, alert.user_id, alert.delay
        );
    }

    /// Removes all alerts for a specific user in a specific room for a game.
    ///
    /// This allows users to unregister from notifications for a particular game
    /// without affecting alerts in other rooms or for other games.
    ///
    /// # Arguments
    ///
    /// * `game_id` - The ID of the game to remove alerts from
    /// * `room_id` - The Matrix room ID where alerts should be removed
    /// * `user_id` - The Matrix user ID whose alerts should be removed
    ///
    /// # Behavior
    ///
    /// - If the game ID doesn't exist: returns without error
    /// - Otherwise: removes all alerts matching the room_id AND user_id
    ///
    /// # Note
    ///
    /// This does NOT abort pending notification tasks. Those should be cleaned up
    /// by the next call to `update_alerts`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::alerts::AlertController;
    ///
    /// # async fn example() {
    /// let controller = AlertController::new("alerts.json".to_string()).await;
    ///
    /// controller.remove_alerts(
    ///     "game_id",
    ///     "!room:example.com",
    ///     "@user:example.com",
    /// ).await;
    /// # }
    /// ```
    pub async fn remove_alerts(&self, game_id: &str, room_id: &str, user_id: &str) {
        let mut alerts_map = self.alerts_map.lock().await;

        let alerts = match alerts_map.get_mut(game_id) {
            Some(alerts) => alerts,
            None => return,
        };

        // Remove all the user alerts in this room for this game
        alerts.retain(|alert| alert.room_id != *room_id || alert.user_id != *user_id);

        info!(
            "unregistered alerts for user {} in room {} for game {}",
            user_id, room_id, game_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmars::{Phase, Player};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::NamedTempFile;
    use tokio::time::{Duration, sleep};

    fn create_test_alert(
        room_id: &str,
        player_id: &str,
        user_id: &str,
        delay: u64,
        notified: bool,
    ) -> Alert {
        Alert {
            room_id: room_id.to_string(),
            player_id: player_id.to_string(),
            user_id: user_id.to_string(),
            notified,
            delay,
            player_url: format!("https://example.com/player?id={}", player_id),
        }
    }

    fn create_test_game(id: &str, waited_players: Vec<&str>) -> Game {
        Game {
            id: id.to_string(),
            phase: Phase::Action,
            spectator_id: "spectator123".to_string(),
            players: vec![
                Player {
                    id: "player1".to_string(),
                    color: "red".to_string(),
                    name: "Player One".to_string(),
                    url: "https://example.com/player1".to_string(),
                },
                Player {
                    id: "player2".to_string(),
                    color: "blue".to_string(),
                    name: "Player Two".to_string(),
                    url: "https://example.com/player2".to_string(),
                },
            ],
            waited_players: waited_players.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[tokio::test]
    async fn test_new_creates_empty_controller() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();

        let controller = AlertController::new(path).await;
        let alerts_map = controller.get_alerts_map().await;

        assert!(alerts_map.is_empty());
    }

    #[tokio::test]
    async fn test_new_loads_existing_alerts() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();

        // Create initial controller and add an alert
        let controller1 = AlertController::new(path.clone()).await;
        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        controller1.add_alert("game1", &alert).await;

        // Manually persist
        let alerts_map = controller1.get_alerts_map().await;
        controller1
            .alert_loader
            .persist_alerts_map(&alerts_map)
            .await;

        // Create new controller - should load the persisted alert
        let controller2 = AlertController::new(path).await;
        let loaded_alerts = controller2.get_alerts_map().await;

        assert_eq!(loaded_alerts.len(), 1);
        assert!(loaded_alerts.contains_key("game1"));
        assert_eq!(loaded_alerts.get("game1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_add_alert_creates_new_game_entry() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        controller.add_alert("game1", &alert).await;

        let alerts_map = controller.get_alerts_map().await;
        assert_eq!(alerts_map.len(), 1);
        assert!(alerts_map.contains_key("game1"));
    }

    #[tokio::test]
    async fn test_add_alert_adds_to_existing_game() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let controller = AlertController::new(path).await;

        let alert1 = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        let alert2 = create_test_alert(
            "!room1:example.com",
            "player2",
            "@user2:example.com",
            120,
            false,
        );

        controller.add_alert("game1", &alert1).await;
        controller.add_alert("game1", &alert2).await;

        let alerts_map = controller.get_alerts_map().await;
        assert_eq!(alerts_map.len(), 1);
        assert_eq!(alerts_map.get("game1").unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_add_alert_replaces_duplicate() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let controller = AlertController::new(path).await;

        // Same room, player, and user - should be considered duplicate
        let alert1 = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        let alert2 = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            120,
            false,
        );

        controller.add_alert("game1", &alert1).await;
        controller.add_alert("game1", &alert2).await;

        let alerts_map = controller.get_alerts_map().await;
        // HashSet doesn't replace on duplicate insert, so only one alert exists
        assert_eq!(alerts_map.get("game1").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_remove_alerts_removes_matching_alerts() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        controller.add_alert("game1", &alert).await;

        controller
            .remove_alerts("game1", "!room1:example.com", "@user1:example.com")
            .await;

        let alerts_map = controller.get_alerts_map().await;
        assert!(alerts_map.get("game1").unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_remove_alerts_keeps_different_users() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let controller = AlertController::new(path).await;

        let alert1 = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        let alert2 = create_test_alert(
            "!room1:example.com",
            "player2",
            "@user2:example.com",
            60,
            false,
        );

        controller.add_alert("game1", &alert1).await;
        controller.add_alert("game1", &alert2).await;

        controller
            .remove_alerts("game1", "!room1:example.com", "@user1:example.com")
            .await;

        let alerts_map = controller.get_alerts_map().await;
        assert_eq!(alerts_map.get("game1").unwrap().len(), 1);

        let remaining = alerts_map.get("game1").unwrap().iter().next().unwrap();
        assert_eq!(remaining.user_id, "@user2:example.com");
    }

    #[tokio::test]
    async fn test_remove_alerts_keeps_different_rooms() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let controller = AlertController::new(path).await;

        let alert1 = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        let alert2 = create_test_alert(
            "!room2:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );

        controller.add_alert("game1", &alert1).await;
        controller.add_alert("game1", &alert2).await;

        controller
            .remove_alerts("game1", "!room1:example.com", "@user1:example.com")
            .await;

        let alerts_map = controller.get_alerts_map().await;
        assert_eq!(alerts_map.get("game1").unwrap().len(), 1);

        let remaining = alerts_map.get("game1").unwrap().iter().next().unwrap();
        assert_eq!(remaining.room_id, "!room2:example.com");
    }

    #[tokio::test]
    async fn test_remove_alerts_nonexistent_game() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let controller = AlertController::new(path).await;

        // Should not panic
        controller
            .remove_alerts("nonexistent", "!room1:example.com", "@user1:example.com")
            .await;

        let alerts_map = controller.get_alerts_map().await;
        assert!(alerts_map.is_empty());
    }

    #[tokio::test]
    async fn test_clean_alerts_removes_nonexistent_games() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert1 = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        let alert2 = create_test_alert(
            "!room1:example.com",
            "player2",
            "@user2:example.com",
            60,
            false,
        );

        controller.add_alert("game1", &alert1).await;
        controller.add_alert("game2", &alert2).await;

        // Create games map with only game1
        let mut games_map = HashMap::new();
        games_map.insert("game1".to_string(), create_test_game("game1", vec![]));

        controller.clean_alerts(&games_map).await;

        let alerts_map = controller.get_alerts_map().await;
        assert_eq!(alerts_map.len(), 1);
        assert!(alerts_map.contains_key("game1"));
        assert!(!alerts_map.contains_key("game2"));
    }

    #[tokio::test]
    async fn test_clean_alerts_keeps_existing_games() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        controller.add_alert("game1", &alert).await;

        let mut games_map = HashMap::new();
        games_map.insert("game1".to_string(), create_test_game("game1", vec![]));

        controller.clean_alerts(&games_map).await;

        let alerts_map = controller.get_alerts_map().await;
        assert_eq!(alerts_map.len(), 1);
        assert!(alerts_map.contains_key("game1"));
    }

    #[tokio::test]
    async fn test_get_alerts_to_fire_triggers_for_waited_player() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        controller.add_alert("game1", &alert).await;

        let mut games_map = HashMap::new();
        games_map.insert(
            "game1".to_string(),
            create_test_game("game1", vec!["player1"]),
        );

        let alerts_to_fire = controller.get_alerts_to_fire(&games_map).await;

        assert_eq!(alerts_to_fire.len(), 1);
        assert_eq!(alerts_to_fire[0].0, "game1");
        assert_eq!(alerts_to_fire[0].1.player_id, "player1");
    }

    #[tokio::test]
    async fn test_get_alerts_to_fire_does_not_trigger_for_non_waited_player() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        controller.add_alert("game1", &alert).await;

        let mut games_map = HashMap::new();
        games_map.insert(
            "game1".to_string(),
            create_test_game("game1", vec!["player2"]),
        );

        let alerts_to_fire = controller.get_alerts_to_fire(&games_map).await;

        assert_eq!(alerts_to_fire.len(), 0);
    }

    #[tokio::test]
    async fn test_get_alerts_to_fire_does_not_trigger_already_notified() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            true,
        );
        controller.add_alert("game1", &alert).await;

        let mut games_map = HashMap::new();
        games_map.insert(
            "game1".to_string(),
            create_test_game("game1", vec!["player1"]),
        );

        let alerts_to_fire = controller.get_alerts_to_fire(&games_map).await;

        assert_eq!(alerts_to_fire.len(), 0);
    }

    #[tokio::test]
    async fn test_get_alerts_to_fire_resets_notified_when_turn_ends() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            true,
        );
        controller.add_alert("game1", &alert).await;

        let mut games_map = HashMap::new();
        games_map.insert(
            "game1".to_string(),
            create_test_game("game1", vec!["player2"]),
        );

        controller.get_alerts_to_fire(&games_map).await;

        let alerts_map = controller.get_alerts_map().await;
        let updated_alert = alerts_map.get("game1").unwrap().iter().next().unwrap();
        assert!(!updated_alert.notified);
    }

    #[tokio::test]
    async fn test_get_alerts_to_fire_marks_as_notified() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        controller.add_alert("game1", &alert).await;

        let mut games_map = HashMap::new();
        games_map.insert(
            "game1".to_string(),
            create_test_game("game1", vec!["player1"]),
        );

        controller.get_alerts_to_fire(&games_map).await;

        let alerts_map = controller.get_alerts_map().await;
        let updated_alert = alerts_map.get("game1").unwrap().iter().next().unwrap();
        assert!(updated_alert.notified);
    }

    #[tokio::test]
    async fn test_update_alerts_fires_callback() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        // Use a very short delay for testing (1 second)
        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            0,
            false,
        );
        controller.add_alert("game1", &alert).await;

        let mut games_map = HashMap::new();
        games_map.insert(
            "game1".to_string(),
            create_test_game("game1", vec!["player1"]),
        );

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        controller
            .update_alerts(&games_map, move |_alert| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        // Wait a bit for the delayed task to execute (delay is 0 minutes = 0 seconds in sleep)
        sleep(Duration::from_millis(10)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_update_alerts_cleans_nonexistent_games() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert1 = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            60,
            false,
        );
        let alert2 = create_test_alert(
            "!room1:example.com",
            "player2",
            "@user2:example.com",
            60,
            false,
        );

        controller.add_alert("game1", &alert1).await;
        controller.add_alert("game2", &alert2).await;

        let mut games_map = HashMap::new();
        games_map.insert("game1".to_string(), create_test_game("game1", vec![]));

        controller.update_alerts(&games_map, |_| {}).await;

        let alerts_map = controller.get_alerts_map().await;
        assert_eq!(alerts_map.len(), 1);
        assert!(alerts_map.contains_key("game1"));
    }

    #[tokio::test]
    async fn test_fire_alert_spawns_delayed_task() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            0,
            false,
        );
        let alerts_to_fire = vec![("game1".to_string(), alert.clone())];

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        controller.fire_alert(alerts_to_fire, move |_alert| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Verify task handle was stored
        assert_eq!(controller.thread_handles_map.len(), 1);

        // Wait for task to execute
        sleep(Duration::from_millis(100)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_fire_alert_aborts_existing_task() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let mut controller = AlertController::new(path).await;

        let alert = create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
            10,
            false,
        );

        // Fire alert twice with same alert
        let alerts_to_fire1 = vec![("game1".to_string(), alert.clone())];
        let alerts_to_fire2 = vec![("game1".to_string(), alert.clone())];

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone1 = Arc::clone(&counter);
        let counter_clone2 = Arc::clone(&counter);

        controller.fire_alert(alerts_to_fire1, move |_alert| {
            counter_clone1.fetch_add(1, Ordering::SeqCst);
        });

        // Fire again immediately - should abort first task
        controller.fire_alert(alerts_to_fire2, move |_alert| {
            counter_clone2.fetch_add(1, Ordering::SeqCst);
        });

        // Only one task handle should exist
        assert_eq!(controller.thread_handles_map.len(), 1);
    }
}
