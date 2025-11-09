//! Matrix client synchronization and event handling.
//!
//! This module provides the [`MatrixSync`] struct for managing the Matrix client's
//! synchronization loop and handling real-time events from the homeserver.
//!
//! # Overview
//!
//! The [`MatrixSync::sync`] method:
//! 1. Performs an initial sync to catch up on offline events (especially invites)
//! 2. Sets up event handlers for auto-joining rooms and message processing
//! 3. Enters a continuous sync loop with automatic token persistence
//!
//! # Example
//!
//! ```no_run
//! use miou::matrix::sync::MatrixSync;
//! use miou::matrix::session::MatrixSession;
//! use matrix_sdk::Client;
//!
//! # async fn example(client: Client, session: MatrixSession) -> Result<(), anyhow::Error> {
//! let matrix_sync = MatrixSync::new(&client, &session);
//!
//! // Start syncing with a message handler
//! matrix_sync.sync(|body, room_id, sender_id, event_id| {
//!     println!("Message from {sender_id} in {room_id}: {body}");
//! }).await?;
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use std::sync::Arc;

use log::{error, info, warn};
use matrix_sdk::{
    Client, LoopCtrl, Room, RoomState,
    config::SyncSettings,
    ruma::{
        api::client::filter::FilterDefinition,
        events::room::{
            member::StrippedRoomMemberEvent,
            message::{MessageType, OriginalSyncRoomMessageEvent},
        },
    },
};
use tokio::time::{Duration, sleep};

use crate::matrix::session::MatrixSession;

/// Manages Matrix client synchronization and event processing.
///
/// This struct wraps a Matrix [`Client`] and handles the complete synchronization
/// lifecycle, including:
/// - Initial sync to catch up on missed events
/// - Continuous sync loop for real-time event processing
/// - Automatic sync token persistence for session continuity
/// - Event handler registration for invites and messages
///
/// # Fields
///
/// * `client` - The authenticated Matrix client for API communication
/// * `session` - The session manager for persisting sync state
pub struct MatrixSync {
    /// The matrix client
    client: Client,
    /// The matrix session
    session: MatrixSession,
}

impl MatrixSync {
    /// Creates a new MatrixSync instance.
    ///
    /// This does not start the synchronization process; call [`MatrixSync::sync`]
    /// to begin syncing.
    ///
    /// # Arguments
    ///
    /// * `client` - The authenticated Matrix client
    /// * `session` - The session manager for token persistence
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::matrix::sync::MatrixSync;
    /// use miou::matrix::session::MatrixSession;
    /// use matrix_sdk::Client;
    ///
    /// # async fn example(client: Client, session: MatrixSession) {
    /// let matrix_sync = MatrixSync::new(&client, &session);
    /// # }
    /// ```
    pub fn new(client: &Client, session: &MatrixSession) -> Self {
        MatrixSync {
            client: client.to_owned(),
            session: session.to_owned(),
        }
    }

    /// Starts the synchronization process and enters an infinite loop.
    ///
    /// This method performs the following sequence:
    /// 1. Sets the bot's display name to "Miou"
    /// 2. Registers an auto-join handler for room invitations
    /// 3. Performs an initial sync to process offline events (especially invites)
    /// 4. Registers a message handler with the provided callback
    /// 5. Enters a continuous sync loop, persisting tokens after each sync
    ///
    /// The sync loop will continue indefinitely until an error occurs or the process
    /// is terminated. Sync tokens are persisted after each successful sync to allow
    /// resumption from the last position if the bot restarts.
    ///
    /// # Arguments
    ///
    /// * `on_message` - Callback invoked for each text message in a joined room.
    ///   Parameters are: `(body, room_id, sender_id, event_id)`
    ///   - `body`: The message text content
    ///   - `room_id`: The room where the message was sent
    ///   - `sender_id`: The user who sent the message
    ///   - `event_id`: The unique event identifier
    ///
    /// # Returns
    ///
    /// Never returns under normal operation. Returns `Err` if sync fails.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Setting the display name fails
    /// - The sync loop encounters a fatal error
    ///
    /// Note: Sync token persistence errors are logged but don't stop the sync process.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::matrix::sync::MatrixSync;
    ///
    /// # async fn example(matrix_sync: MatrixSync) -> Result<(), anyhow::Error> {
    /// // Simple message logger
    /// matrix_sync.sync(|body, room_id, sender, event_id| {
    ///     println!("[{room_id}] {sender}: {body}");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sync<F>(&self, on_message: F) -> Result<()>
    where
        F: Fn(String, String, String, String) + Send + Sync + 'static + Clone,
    {
        info!("start syncing");

        // Auto join rooms when invited
        self.client.add_event_handler(auto_join_rooms);

        // Enable room members lazy-loading
        // See <https://spec.matrix.org/v1.6/client-server-api/#lazy-loading-room-members>.
        let filter = FilterDefinition::with_lazy_loading();
        let mut sync_settings = SyncSettings::default().filter(filter.into());

        // Get the last sync token from the session if it exists
        if let Some(sync_token) = self.session.get_sync_token() {
            sync_settings = sync_settings.token(sync_token);
        }

        // First sync to only get the invitation when the bot is offline
        let response = self.client.sync_once(sync_settings.clone()).await.unwrap();
        loop {
            match self.client.sync_once(sync_settings.clone()).await {
                Ok(response) => {
                    // update the sync token in the session
                    sync_settings = sync_settings.token(response.next_batch.clone());
                    // persist the sync token to disk
                    if let Err(err) = self.session.persist_sync_token(response.next_batch).await {
                        error!("failed to persist sync token: {:?}", err);
                    }
                    break;
                }
                Err(error) => {
                    error!("an error occurred during initial sync: {error}");
                    error!("trying again…");
                }
            }
        }

        let on_message_arc = Arc::new(on_message);

        // Listen to incoming room messages. Because we are listening after the sync_once, we only get new messages.
        self.client.add_event_handler({
            let on_message = Arc::clone(&on_message_arc);
            move |event: OriginalSyncRoomMessageEvent, room: Room| async move {
                on_room_message(event, room, &on_message).await
            }
        });

        // Since we called `sync_once` before we entered our sync loop we must pass
        // that sync token to `sync_with_result_callback`
        sync_settings = sync_settings.token(response.next_batch);

        self.client
            .sync_with_result_callback(sync_settings, |sync_result| async move {
                let response = sync_result?;

                // We persist the token each time to be able to restore our session
                if let Err(err) = self.session.persist_sync_token(response.next_batch).await {
                    error!("failed to persist sync token: {:?}", err);
                }

                Ok(LoopCtrl::Continue)
            })
            .await?;

        Ok(())
    }
}

/// Automatically joins rooms when the bot receives an invitation.
///
/// # Arguments
///
/// * `room_member` - The stripped room member event containing the invite
/// * `client` - The Matrix client to use for joining
/// * `room` - The room to join
///
/// # References
///
/// See <https://github.com/matrix-org/synapse/issues/4345> for the Synapse issue
/// that necessitates the retry logic.
async fn auto_join_rooms(room_member: StrippedRoomMemberEvent, client: Client, room: Room) {
    let Some(user_id) = client.user_id() else {
        warn!("could not get user id from client");
        return;
    };

    // Ignore if the invite is not for us
    if room_member.state_key != user_id {
        return;
    }

    tokio::spawn(async move {
        info!("auto joining room {}", room.room_id());
        let mut delay = 2;

        while let Err(err) = room.join().await {
            // retry autojoin due to synapse sending invites, before the
            // invited user can join for more information see
            // https://github.com/matrix-org/synapse/issues/4345
            error!(
                "failed to join room {} ({err:?}), retrying in {delay}s",
                room.room_id()
            );

            sleep(Duration::from_secs(delay)).await;
            delay *= 2;

            if delay > 3600 {
                error!("can't join room {} ({err:?})", room.room_id());
                break;
            }
        }
        info!("successfully joined room {}", room.room_id());
    });
}

/// Handles incoming room messages and delegates to the user callback.
///
/// This internal function:
/// 1. Filters out messages from non-joined rooms
/// 2. Extracts text content from message events
/// 3. Invokes the user-provided callback with message details
///
/// Non-text messages (images, files, etc.) are silently ignored.
///
/// # Arguments
///
/// * `event` - The room message event from the sync stream
/// * `room` - The room where the message was sent
/// * `on_message` - The user-provided callback to invoke
///
/// # Type Parameters
///
/// * `F` - The callback function type with signature:
///   `Fn(String, String, String, String)` for `(body, room_id, sender, event_id)`
async fn on_room_message<F>(event: OriginalSyncRoomMessageEvent, room: Room, on_message: &Arc<F>)
where
    F: Fn(String, String, String, String) + Send + Sync + 'static,
{
    // Ignore messages from non-joined rooms
    if room.state() != RoomState::Joined {
        return;
    }

    // Only handle text messages
    let MessageType::Text(text_content) = event.content.msgtype else {
        return;
    };

    on_message(
        text_content.body,
        room.room_id().to_string(),
        event.sender.to_string(),
        event.event_id.to_string(),
    );
}
