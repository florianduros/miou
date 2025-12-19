//! Terraforming Mars game integration and API client.
//!
//! This module provides integration with the Terraforming Mars online platform,
//! handling API communication, data synchronization, and game state management.
//!
//! # Modules
//!
//! - `requester` - HTTP client for making API requests to the Terraforming Mars server
//! - `response_structs` - Internal data structures for API responses
//! - `structs` - Public data structures representing games, players, and game states
//! - `sync` - Synchronization logic for fetching and updating game data
//!
//! # Examples
//!
//! ```no_run
//! use miou::tmars::TMarsRequester;
//!
//! let requester = TMarsRequester::new(
//!     "server_id".to_string(),
//!     "https://tmars.example.com".to_string()
//! );
//! // Fetch games and sync state
//! ```

mod requester;
mod response_structs;
mod structs;
mod sync;

pub use crate::tmars::requester::TMarsRequester;
pub use crate::tmars::structs::Game;
#[cfg(test)]
pub use crate::tmars::structs::{Phase, Player};
pub use crate::tmars::sync::TMarsSync;

/// Errors that can occur during synchronization with the TMars API.
///
/// This enum represents the different error conditions that can arise when
/// synchronizing game data with the Terraforming Mars server.
///
/// # Variants
///
/// * `AccessError` - Authentication or authorization failure (HTTP 401/403)
#[derive(Debug)]
pub enum SyncError {
    /// Authentication or authorization failure.
    ///
    /// Returned when the TMars API responds with HTTP 401 (Unauthorized) or
    /// HTTP 403 (Forbidden) status codes.
    AccessError,
}
