//! Matrix client wrapper for bot messaging and synchronization.
//!
//! This module provides a high-level [`MatrixClient`] interface that wraps the
//! Matrix SDK client and handles message sending, synchronization, and session management.

use log::{error, info, warn};
use matrix_sdk::{
    Client,
    ruma::{
        EventId, RoomId, UserId,
        events::{
            Mentions,
            room::message::{AddMentions, ForwardThread, ReplyMetadata, RoomMessageEventContent},
        },
    },
};

use crate::matrix::{
    UserCredentials, encryption::setup_client, session::MatrixSession, sync::MatrixSync,
};

/// High-level Matrix client for bot messaging operations.
///
/// Manages a Matrix SDK client with synchronization capabilities and provides
/// convenient methods for sending messages, mentions, and replies.
pub struct MatrixClient {
    /// Synchronization service for handling real-time events
    matrix_sync: MatrixSync,
    /// Underlying Matrix SDK client
    client: Client,
}

impl MatrixClient {
    /// Creates and initializes a new Matrix client with full encryption setup.
    ///
    /// This method performs the complete initialization workflow:
    /// 1. Creates or restores a Matrix session from the session path
    /// 2. Sets up the Matrix client with encryption (via [`setup_client`])
    /// 3. Initializes the synchronization service
    ///
    /// # Arguments
    ///
    /// * `user_credentials` - User credentials containing user ID, password, and passphrase
    /// * `session_path` - Directory path for storing session data and SQLite database
    /// * `avatar_bytes` - Byte slice containing the PNG image data for the bot's avatar
    ///
    /// # Returns
    ///
    /// A fully configured [`MatrixClient`] ready for messaging and synchronization.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Session creation fails
    /// - Client setup or encryption initialization fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use miou::matrix::{UserCredentials, client::MatrixClient};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), anyhow::Error> {
    /// let credentials = UserCredentials {
    ///     user_id: "@bot:example.com".to_string(),
    ///     password: "secure_password".to_string(),
    ///     passphrase: "secure_passphrase".to_string(),
    /// };
    ///
    /// let avatar_bytes = include_bytes!("../../assets/miou.png");
    /// let client = MatrixClient::new(&credentials, "./session", avatar_bytes).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(
        user_credentials: &UserCredentials,
        session_path: &str,
        avatar_bytes: &[u8],
    ) -> Result<Self, anyhow::Error> {
        let matrix_session_result = MatrixSession::new(session_path).await;
        if matrix_session_result.is_err() {
            error!("failed to create matrix session");
            return Err(anyhow::anyhow!("failed to create matrix session"));
        }
        let matrix_session = matrix_session_result.unwrap();

        let client_result = setup_client(user_credentials, &matrix_session).await;
        if client_result.is_err() {
            error!("failed to setup matrix client");
            return Err(anyhow::anyhow!("failed to setup matrix client"));
        }
        let client = client_result.unwrap();

        // Set display name
        client.account().set_display_name(Some("Miou")).await?;

        // Set avatar if not already set
        if client.account().get_avatar_url().await?.is_none()
            && let Err(e) = client
                .account()
                .upload_avatar(&mime::IMAGE_PNG, avatar_bytes.to_vec())
                .await
        {
            warn!("failed to upload avatar: {:?}", e);
        }

        let matrix_sync = MatrixSync::new(&client, &matrix_session);

        Ok(MatrixClient {
            matrix_sync,
            client,
        })
    }

    /// Starts the Matrix synchronization loop.
    ///
    /// This method begins syncing with the Matrix server and invokes the provided
    /// callback for each incoming text message. The sync loop runs indefinitely
    /// and automatically handles:
    /// - Auto-joining rooms on invitation
    /// - Filtering for text messages in joined rooms
    /// - Persisting sync tokens for continuity
    ///
    /// # Arguments
    ///
    /// * `on_message` - Callback invoked for each text message with parameters:
    ///   - `body`: The message text content
    ///   - `room_id`: The room where the message was sent
    ///   - `sender_id`: The user who sent the message
    ///   - `event_id`: The unique event identifier
    ///
    /// # Returns
    ///
    /// Never returns under normal operation. Returns `Ok(())` if sync ends gracefully.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use miou::matrix::client::MatrixClient;
    /// # async fn example(client: MatrixClient) -> Result<(), anyhow::Error> {
    /// client.sync(|body, room_id, sender_id, event_id| {
    ///     println!("[{room_id}] {sender_id}: {body}");
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn sync<F>(&self, on_message: F) -> Result<(), anyhow::Error>
    where
        F: Fn(String, String, String, String) + Send + Sync + 'static + Clone,
    {
        match self.matrix_sync.sync(on_message).await {
            Ok(_) => info!("matrix sync ended successfully"),
            Err(e) => error!("matrix sync ended with error: {:?}", e),
        }

        Ok(())
    }

    /// Sends a message with a mention to a specific user.
    ///
    /// The message body is formatted as Markdown and includes a mention of the
    /// specified user, which will trigger a notification for them.
    ///
    /// # Arguments
    ///
    /// * `room_id` - The Matrix room ID where the message should be sent
    /// * `body` - The message content (supports Markdown formatting)
    /// * `sender_id` - The user ID to mention in the message
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use miou::matrix::client::MatrixClient;
    /// # async fn example(client: MatrixClient) {
    /// client.send_mention(
    ///     "!room:example.com".to_string(),
    ///     "Hello! Your turn is ready.".to_string(),
    ///     "@user:example.com".to_string(),
    /// ).await;
    /// # }
    /// ```
    pub async fn send_mention(&self, room_id: &str, body: &str, sender_id: &str) {
        let sender = UserId::parse(sender_id).unwrap();
        let content = RoomMessageEventContent::text_markdown(body)
            .add_mentions(Mentions::with_user_ids([sender]));

        println!("body {}", content.body());

        self.send(room_id, content).await;
    }

    /// Sends a threaded reply to a specific message.
    ///
    /// Creates a reply to an existing message, maintaining proper thread context
    /// in the Matrix room. The message body is formatted as Markdown.
    ///
    /// # Arguments
    ///
    /// * `room_id` - The Matrix room ID where the reply should be sent
    /// * `sender_id` - The user ID of the original message sender
    /// * `event_id` - The event ID of the message being replied to
    /// * `body` - The reply content (supports Markdown formatting)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use miou::matrix::client::MatrixClient;
    /// # async fn example(client: MatrixClient) {
    /// client.send_reply(
    ///     "!room:example.com".to_string(),
    ///     "@user:example.com".to_string(),
    ///     "$event123:example.com".to_string(),
    ///     "Thanks for your message!".to_string(),
    /// ).await;
    /// # }
    /// ```
    pub async fn send_reply(&self, room_id: &str, sender_id: &str, event_id: &str, body: &str) {
        let sender = UserId::parse(sender_id).unwrap();
        let event = EventId::parse(event_id).unwrap();

        let content = RoomMessageEventContent::text_markdown(body).make_reply_to(
            ReplyMetadata::new(&event, &sender, None),
            ForwardThread::No,
            AddMentions::No,
        );

        self.send(room_id, content).await;
    }

    /// Internal helper to send message content to a room.
    ///
    /// # Arguments
    ///
    /// * `room_id` - The Matrix room ID
    /// * `content` - The pre-formatted message content
    async fn send(&self, room_id: &str, content: RoomMessageEventContent) {
        let room_id_obj = RoomId::parse(room_id).unwrap();

        if let Some(room) = self.client.get_room(&room_id_obj)
            && let Err(e) = room.send(content).await
        {
            error!("Failed to send message: {:?}", e);
        }
    }
}
