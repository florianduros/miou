//! Alert management system for Terraforming Mars game notifications.
//!
//! This module provides a complete alert system that notifies Matrix users when it's
//! their turn in a Terraforming Mars game. The system consists of three main components:
//!
//! - [`Alert`]: Represents a single user notification preference
//! - [`AlertController`]: Manages alert lifecycle, notification scheduling, and persistence
//! - [`AlertLoader`]: Handles loading and saving alerts to disk
//!
//! # Architecture
//!
//! The alert system uses a map to track alerts by game ID. Each game can have
//! multiple alerts from different users in different Matrix rooms. The [`AlertController`]
//! monitors game state and spawns delayed notification tasks when a player's turn arrives.
//!
//! # Example Usage
//!
//! ```no_run
//! use miou::alerts::{Alert, AlertController};
//! use std::collections::HashMap;
//!
//! # async fn example() {
//! // Initialize the alert controller
//! let mut controller = AlertController::new("alerts.json".to_string()).await;
//!
//! // Start periodic persistence
//! controller.start_persistence_task();
//!
//! // Add an alert
//! let alert = Alert {
//!     room_id: "!room:example.com".to_string(),
//!     player_id: "player123".to_string(),
//!     user_id: "@user:example.com".to_string(),
//!     notified: false,
//!     delay: 60,
//!     player_url: "https://example.com/player?id=player123".to_string(),
//! };
//! controller.add_alert("game_id", &alert).await;
//!
//! // Later, remove alerts for a user
//! controller.remove_alerts("game_id", "!room:example.com", "@user:example.com").await;
//! # }
//! ```

mod alert;
mod alert_controller;
mod alert_loader;

pub use crate::alerts::alert_loader::AlertLoader;
pub use crate::alerts::{alert::Alert, alert_controller::AlertController};
