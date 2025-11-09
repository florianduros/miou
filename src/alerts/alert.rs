//! Alert management for Terraforming Mars game notifications.
//!
//! This module provides the [`Alert`] struct for tracking user notification
//! preferences for Terraforming Mars games. Alerts notify users when it's
//! their turn to play after a specified delay.

use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// Represents an alert registered by a Matrix user for a Terraforming Mars player.
///
/// An alert associates a Matrix user with a Terraforming Mars player in a specific
/// room, tracking notification preferences and state. When the player's turn arrives,
/// the user will be notified in the specified Matrix room after the configured delay.
///
/// # Equality and Hashing
///
/// Two alerts are considered equal if they have the same `room_id`, `player_id`,
/// and `user_id`, regardless of their `notified` status or `delay` value. This
/// allows detecting duplicate alert registrations.
///
/// # Examples
///
/// ```
/// # use miou::alert::Alert;
/// let alert = Alert {
///     room_id: "!room:example.com".to_string(),
///     player_id: "player123".to_string(),
///     user_id: "@user:example.com".to_string(),
///     notified: false,
///     delay: 60, // 60 minutes
///     player_url: "https://example.com/player?id=player123".to_string(),
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Alert {
    /// Matrix room ID where the user registered the alert.
    ///
    /// This is where the notification will be sent.
    pub room_id: String,
    /// Terraforming Mars player ID to monitor.
    ///
    /// The alert triggers when this player's turn arrives.
    pub player_id: String,
    /// Matrix user ID who registered the alert.
    ///
    /// This user will receive the notification.
    pub user_id: String,
    /// Whether the user has been notified for this alert.
    ///
    /// Set to `true` after sending the notification to prevent duplicates.
    pub notified: bool,
    /// Delay in minutes before notifying the user.
    ///
    /// When the player's turn arrives, the bot waits this many minutes
    /// before sending the notification. Must be between 1 and 10,080 (1 week).
    pub delay: u64,
    /// URL to the player's game page.
    ///
    /// This URL points to the player's view of the game on the Terraforming Mars server.
    pub player_url: String,
}

/// Implementation of partial equality for alerts.
///
/// Two alerts are equal if they share the same room, player, and user,
/// regardless of notification status or delay. This prevents users from
/// registering duplicate alerts for the same player in the same room.
impl PartialEq for Alert {
    fn eq(&self, other: &Self) -> bool {
        self.room_id == other.room_id
            && self.user_id == other.user_id
            && self.player_id == other.player_id
    }
}

/// Implementation of full equality for alerts.
///
/// Required for using `Alert` in collections that need `Eq`.
impl Eq for Alert {}

/// Implementation of hashing for alerts.
///
/// Hashes based on `room_id`, `player_id`, and `user_id` only, consistent
/// with the equality implementation. This allows alerts to be used in
/// hash-based collections like `HashSet` and `HashMap`.
impl Hash for Alert {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.room_id.hash(state);
        self.player_id.hash(state);
        self.user_id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use std::hash::DefaultHasher;

    use super::*;

    /// Helper function to calculate hash of an alert for testing.
    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    #[test]
    fn test_is_equal() {
        let al1 = Alert {
            room_id: "room1".to_string(),
            player_id: "player1".to_string(),
            user_id: "user1".to_string(),
            notified: false,
            delay: 10,
            player_url: "http://example.com/player1".to_string(),
        };

        let al2 = Alert {
            room_id: "room1".to_string(),
            player_id: "player1".to_string(),
            user_id: "user1".to_string(),
            notified: true,
            delay: 20,
            player_url: "http://example.com/player1".to_string(),
        };

        assert!(al1 == al2);
        assert!(calculate_hash(&al1) == calculate_hash(&al2))
    }

    #[test]
    fn test_is_not_equal() {
        let al1 = Alert {
            room_id: "room1".to_string(),
            player_id: "player1".to_string(),
            user_id: "user1".to_string(),
            notified: false,
            delay: 10,
            player_url: "http://example.com/player1".to_string(),
        };

        let al2 = Alert {
            room_id: "room1".to_string(),
            player_id: "player2".to_string(),
            user_id: "user1".to_string(),
            notified: true,
            delay: 20,
            player_url: "http://example.com/player2".to_string(),
        };

        assert!(al1 != al2);
        assert!(calculate_hash(&al1) != calculate_hash(&al2))
    }
}
