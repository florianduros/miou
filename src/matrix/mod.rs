//! Matrix protocol integration for the bot.
//!
//! This module provides a complete Matrix client implementation with support for:
//! - End-to-end encryption
//! - Session management and persistence
//! - Real-time event synchronization
//! - Message sending with mentions and replies
//!
//! # Architecture
//!
//! The module is structured around the [`client::MatrixClient`] which coordinates:
//! - **Encryption**: Cross-signing and key management via the encryption submodule
//! - **Session**: Login, logout, and session restoration via the session submodule
//! - **Sync**: Real-time event handling and room synchronization via the sync submodule
//!
//! # Examples
//!
//! ```no_run
//! use miou::matrix::{UserCredentials, client::MatrixClient};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let credentials = UserCredentials {
//!     user_id: "@bot:example.com".to_string(),
//!     password: "password".to_string(),
//!     passphrase: "recovery_phrase".to_string(),
//! };
//!
//! let client = MatrixClient::new(&credentials, "./session".to_string()).await?;
//! # Ok(())
//! # }
//! ```

mod client;
mod encryption;
mod session;
mod sync;

pub use crate::matrix::client::MatrixClient;

/// User credentials for a Matrix account
#[derive(Debug, Clone)]
pub struct UserCredentials {
    /// User ID of the matrix account
    pub user_id: String,
    /// Password of the matrix account
    pub password: String,
    /// Passphrase to recover the matrix account secrets
    pub passphrase: String,
}
