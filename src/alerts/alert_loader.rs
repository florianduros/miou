//! Alert persistence layer for loading and saving alerts to disk.
//!
//! This module provides the [`AlertLoader`] for persisting alert data between
//! bot restarts. Alerts are serialized to JSON and stored in a file.

use std::collections::{HashMap, HashSet};

use log::{error, info, warn};
use tokio::fs;

use crate::alerts::alert::Alert;

/// Handles loading and persisting alerts to disk.
///
/// The `AlertLoader` manages serialization and deserialization of the alerts map,
/// providing fault-tolerant file I/O operations. If loading fails (file missing or
/// corrupted), it gracefully returns an empty map rather than panicking.
///
/// # Examples
///
/// ```no_run
/// use miou::alerts::AlertLoader;
///
/// # async fn example() {
/// let loader = AlertLoader::new("alerts.json".to_string());
///
/// // Load existing alerts or get an empty map
/// let alerts_map = loader.load().await;
///
/// // Later, persist the alerts
/// loader.persist_alerts_map(&alerts_map).await;
/// # }
/// ```
#[derive(Clone)]
pub struct AlertLoader {
    /// Path to the JSON file where alerts are stored.
    path: String,
}

impl AlertLoader {
    /// Creates a new `AlertLoader` for the specified file path.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path where alerts will be loaded from and saved to
    ///
    /// # Examples
    ///
    /// ```
    /// use miou::alerts::AlertLoader;
    ///
    /// let loader = AlertLoader::new("alerts.json".to_string());
    /// ```
    pub fn new(path: String) -> Self {
        AlertLoader { path }
    }

    /// Loads alerts from disk.
    ///
    /// Reads the alerts file and deserializes it into a map of game IDs to alert sets.
    /// If the file doesn't exist or cannot be deserialized, returns an empty map.
    ///
    /// # Returns
    ///
    /// A `HashMap` mapping game IDs to sets of alerts for each game.
    ///
    /// # Error Handling
    ///
    /// - If the file doesn't exist: logs a warning and returns an empty map
    /// - If deserialization fails: logs an error and returns an empty map
    ///
    /// This ensures the bot can always start, even with corrupted or missing alert data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::alerts::AlertLoader;
    ///
    /// # async fn example() {
    /// let loader = AlertLoader::new("alerts.json".to_string());
    /// let alerts = loader.load().await;
    /// println!("Loaded {} games with alerts", alerts.len());
    /// # }
    /// ```
    pub async fn load(&self) -> HashMap<String, HashSet<Alert>> {
        let Ok(serialized_alerts_map) = fs::read_to_string(&self.path).await else {
            warn!("no persisted alerts found, starting with an empty alerts map");
            return HashMap::new();
        };

        let Ok(alerts_map) = serde_json::from_str(&serialized_alerts_map) else {
            error!("failed to deserialize persisted alerts, starting with an empty alerts map");
            return HashMap::new();
        };

        info!("loaded persisted alerts {}", serialized_alerts_map);

        alerts_map
    }

    /// Persists the alerts map to disk.
    ///
    /// Serializes the entire alerts map to JSON and writes it to the configured file path.
    /// This operation is atomic - either the entire write succeeds or it fails without
    /// partially corrupting the file.
    ///
    /// # Arguments
    ///
    /// * `alerts_map` - Reference to the alerts map to persist
    ///
    /// # Error Handling
    ///
    /// - If serialization fails: logs an error and returns without writing
    /// - If file write fails: logs an error with details
    ///
    /// Errors are logged but not propagated, allowing the bot to continue operating
    /// even if persistence fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::alerts::AlertLoader;
    /// use std::collections::HashMap;
    ///
    /// # async fn example() {
    /// let loader = AlertLoader::new("alerts.json".to_string());
    /// let alerts_map = loader.load().await;
    ///
    /// // ... modify alerts_map ...
    ///
    /// loader.persist_alerts_map(&alerts_map).await;
    /// # }
    /// ```
    pub async fn persist_alerts_map(&self, alerts_map: &HashMap<String, HashSet<Alert>>) {
        let serialized_alerts_map = match serde_json::to_string(alerts_map) {
            Ok(serialized) => serialized,
            Err(e) => {
                error!("failed to serialize alerts map: {}", e);
                return;
            }
        };

        if let Err(e) = fs::write(&self.path, &serialized_alerts_map).await {
            error!("failed to persist alerts map: {}", e);
            return;
        }

        info!("persisted alerts");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn create_test_alert(room_id: &str, player_id: &str, user_id: &str) -> Alert {
        Alert {
            room_id: room_id.to_string(),
            player_id: player_id.to_string(),
            user_id: user_id.to_string(),
            notified: false,
            delay: 60,
            player_url: format!("https://example.com/player?id={}", player_id),
        }
    }

    #[tokio::test]
    async fn test_load_nonexistent_file_returns_empty_map() {
        let loader = AlertLoader::new("nonexistent_file.json".to_string());
        let alerts_map = loader.load().await;

        assert!(alerts_map.is_empty());
    }

    #[tokio::test]
    async fn test_persist_and_load_empty_map() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let loader = AlertLoader::new(path.clone());

        let empty_map: HashMap<String, HashSet<Alert>> = HashMap::new();
        loader.persist_alerts_map(&empty_map).await;

        let loaded_map = loader.load().await;
        assert!(loaded_map.is_empty());
    }

    #[tokio::test]
    async fn test_persist_and_load_single_alert() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let loader = AlertLoader::new(path.clone());

        let mut alerts_map: HashMap<String, HashSet<Alert>> = HashMap::new();
        let mut alerts = HashSet::new();
        alerts.insert(create_test_alert(
            "!room1:example.com",
            "player1",
            "@user1:example.com",
        ));
        alerts_map.insert("game1".to_string(), alerts);

        loader.persist_alerts_map(&alerts_map).await;

        let loaded_map = loader.load().await;
        assert_eq!(loaded_map.len(), 1);
        assert!(loaded_map.contains_key("game1"));

        let loaded_alerts = loaded_map.get("game1").unwrap();
        assert_eq!(loaded_alerts.len(), 1);
    }

    #[tokio::test]
    async fn test_load_preserves_alert_properties() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let loader = AlertLoader::new(path.clone());

        let mut alerts_map: HashMap<String, HashSet<Alert>> = HashMap::new();
        let mut alerts = HashSet::new();

        let alert = Alert {
            room_id: "!room1:example.com".to_string(),
            player_id: "player1".to_string(),
            user_id: "@user1:example.com".to_string(),
            notified: true,
            delay: 120,
            player_url: "https://example.com/player?id=player1".to_string(),
        };
        alerts.insert(alert);
        alerts_map.insert("game1".to_string(), alerts);

        loader.persist_alerts_map(&alerts_map).await;

        let loaded_map = loader.load().await;
        let loaded_alert = loaded_map.get("game1").unwrap().iter().next().unwrap();

        assert_eq!(loaded_alert.room_id, "!room1:example.com");
        assert_eq!(loaded_alert.player_id, "player1");
        assert_eq!(loaded_alert.user_id, "@user1:example.com");
        assert!(loaded_alert.notified);
        assert_eq!(loaded_alert.delay, 120);
        assert_eq!(
            loaded_alert.player_url,
            "https://example.com/player?id=player1"
        );
    }

    #[tokio::test]
    async fn test_load_corrupted_json_returns_empty_map() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();

        // Write invalid JSON
        fs::write(&path, "{ this is not valid json ").await.unwrap();

        let loader = AlertLoader::new(path);
        let alerts_map = loader.load().await;

        assert!(alerts_map.is_empty());
    }
}
