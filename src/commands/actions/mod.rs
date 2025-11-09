//! Command action handlers.
//!
//! Individual handler functions for each bot command. Each handler receives a
//! [`CommandContext`](crate::commands::CommandContext), processes the command,
//! and returns a [`CommandResult`](crate::commands::CommandResult).
//!
//! # Handler Pattern
//!
//! Handlers follow a consistent pattern:
//! 1. Receive context with runtime state (games, alerts, user/room IDs)
//! 2. Validate and process the command
//! 3. Return a result with Markdown response and optional state changes
//!
//! # Available Handlers
//!
//! - [`handle_help`] - Display help information
//! - [`handle_games`] - List ongoing games with players
//! - [`handle_alerts`] - Show user's alert subscriptions
//! - [`handle_register`] - Register new turn notification alert
//! - [`handle_unregister`] - Remove alert subscriptions for a game
//!
//! # State Changes
//!
//! Handlers don't modify state directly. Instead, they return state change requests
//! via `alert_to_add` or `alerts_to_remove` in the [`CommandResult`](crate::commands::CommandResult).

mod alerts;
mod games;
mod help;
mod register;
mod unregister;

pub use crate::commands::actions::{
    alerts::handle_alerts, games::handle_games, help::handle_help, register::handle_register,
    unregister::handle_unregister,
};
