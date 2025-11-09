//! Bot module for managing Matrix-TMars integration.
//!
//! This module provides the main [`Bot`] implementation that connects a Matrix client
//! with the TMars (Terraforming Mars) game server. It orchestrates the complete bot
//! lifecycle including game synchronization, command processing, alert management,
//! and player notifications.
//!
//! # Overview
//!
//! The Miou bot allows Matrix users to register for turn notifications in Terraforming
//! Mars games. When it's a player's turn, the bot sends a Matrix mention to the registered
//! user after a configurable delay.
//!
//! # Architecture
//!
//! The bot operates with three main concurrent tasks:
//!
//! 1. **TMars Sync Task**: Periodically polls the TMars game server for state updates,
//!    checks registered alerts against current game state, and triggers notifications
//!    when players' turns arrive.
//!
//! 2. **Matrix Sync Task**: Continuously listens for Matrix messages, parses user
//!    commands, executes them, and sends responses back to Matrix rooms.
//!
//! 3. **Alert Persistence Task**: Periodically saves the alerts map to disk to ensure
//!    alerts survive bot restarts.
//!
//! # Command Processing Flow
//!
//! ```text
//! Matrix Message → Parse Command → Validate → Execute → Update Alerts → Send Response
//! ```
//!
//! # Supported Commands
//!
//! - `register` - Register an alert for a player in a game
//! - `unregister` - Remove alerts for a game
//! - `list` - List all registered alerts
//! - `help` - Display help information
//!
//! # Example
//!
//! ```no_run
//! # use miou::bot::Bot;
//! # use miou::config::Config;
//! # use miou::Args;
//! # async fn run() -> Result<(), anyhow::Error> {
//! let config = Config::load("config.toml")?;
//! let args = Args::parse();
//!
//! // Create and start the bot
//! let bot = Bot::new(config, args).await?;
//! bot.start().await; // Runs indefinitely
//! # Ok(())
//! # }
//! ```

use log::info;
use tokio::time;

use crate::{
    Args,
    alerts::{Alert, AlertController},
    commands::{CommandContext, CommandParseError, Commander},
    config::Config,
    matrix::{MatrixClient, UserCredentials},
    tmars::{TMarsRequester, TMarsSync},
    utils::get_path,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

/// Context for processing a Matrix message.
///
/// Groups together all the information needed to process a single Matrix message
/// and execute commands.
struct MessageContext {
    /// The message body text
    body: String,
    /// The Matrix room ID where the message was sent
    room_id: String,
    /// The Matrix user ID who sent the message
    sender_id: String,
    /// The Matrix event ID of the message
    event_id: String,
    /// Thread-safe reference to the Matrix client
    matrix_client: Arc<MatrixClient>,
    /// Thread-safe reference to the TMars sync service
    tmars_sync: Arc<Mutex<TMarsSync<TMarsRequester>>>,
    /// Thread-safe reference to the alert controller for managing notifications
    alert_controller: Arc<Mutex<AlertController>>,
    /// Thread-safe reference to the command handler
    commander: Arc<Commander>,
}

/// Main bot structure that integrates Matrix messaging with TMars game server.
///
/// The `Bot` orchestrates the complete lifecycle of the Miou bot, managing three primary
/// responsibilities:
///
/// 1. **Game Synchronization** - Periodically polls the TMars server for game state updates
/// 2. **Message Processing** - Listens to Matrix rooms and processes user commands
/// 3. **Alert Management** - Tracks user notification preferences and triggers delayed notifications
///
/// # Architecture
///
/// The bot operates with three concurrent async tasks:
///
/// - **TMars Sync Task**: Runs on a timer (configured by `polling_interval`), fetching
///   game states from the TMars server and updating the alert controller with current
///   game state to trigger notifications
/// - **Matrix Sync Task**: Continuously listens for Matrix messages, parses commands,
///   executes them, and sends responses back to users
/// - **Alert Persistence Task**: Runs on a timer, periodically saving the alerts map
///   to disk to ensure alerts survive bot restarts
///
/// # Alert Management
///
/// The bot uses an [`AlertController`] to manage notification preferences. Alerts are:
///
/// - **Added** when users run the `register` command for a player in a game
/// - **Removed** when users run the `unregister` command or when games no longer exist
/// - **Triggered** when a monitored player's turn arrives, after a user-configured delay
/// - **Persisted** automatically to disk every minute to survive bot restarts
///
/// # Thread Safety
///
/// All shared state (`matrix_client`, `tmars_sync`, `alert_controller`, `commander`) is
/// wrapped in `Arc` for safe sharing across async tasks. Mutable state uses `Mutex` for
/// interior mutability, ensuring thread-safe concurrent access.
///
/// # Examples
///
/// ```no_run
/// # use miou::bot::Bot;
/// # use miou::config::{Config, TMars, Matrix};
/// # #[derive(Debug)]
/// # struct Args {
/// #     config: String,
/// #     session_path: String,
/// # }
/// # async fn example() -> Result<(), anyhow::Error> {
/// let config = Config {
///     tmars: TMars {
///         url: "https://tmars.example.com".to_string(),
///         server_id: "server1".to_string(),
///         polling_interval: 120,
///     },
///     matrix: Matrix {
///         user_id: "@bot:example.com".to_string(),
///         password: "secret".to_string(),
///         passphrase: "passphrase".to_string(),
///     },
/// };
///
/// let args = Args {
///     config: "config.toml".to_string(),
///     session_path: "./session".to_string(),
/// };
///
/// let bot = Bot::new(config, args).await?;
/// bot.start().await; // Runs indefinitely
/// # Ok(())
/// # }
/// ```
pub struct Bot {
    /// Matrix client for sending and receiving messages.
    ///
    /// Handles all communication with the Matrix server, including:
    /// - Syncing to receive new messages
    /// - Sending command responses
    /// - Sending turn notifications
    matrix_client: Arc<MatrixClient>,

    /// TMars synchronization service for fetching game states.
    ///
    /// Wrapped in `Mutex` because it maintains internal state (last fetched games)
    /// that is updated during each sync operation.
    tmars_sync: Arc<Mutex<TMarsSync<TMarsRequester>>>,

    /// Polling interval in seconds for syncing with the TMars server.
    ///
    /// Determines how frequently the bot checks for game state updates.
    polling_interval: u64,

    /// Alert controller for managing game notifications.
    ///
    /// Wrapped in `Mutex` because it maintains internal state including:
    ///
    /// - The alerts map (game ID -> set of alerts)
    /// - Thread handles for pending notification tasks
    ///
    /// These are updated during alert registration/removal and when notifications fire.
    alert_controller: Arc<Mutex<AlertController>>,

    /// Command parser and executor.
    ///
    /// Handles parsing Matrix messages into structured commands and routing
    /// them to appropriate handlers. Stateless and can be safely shared.
    commander: Arc<Commander>,
}

impl Bot {
    /// Creates a new Bot instance from configuration and command line arguments.
    ///
    /// This constructor initializes all bot components including the Matrix client,
    /// TMars requester, and command parser. It performs the Matrix login if no valid
    /// session exists, or restores the previous session if available.
    ///
    /// # Arguments
    ///
    /// * `config` - TOML configuration loaded from file containing:
    ///   - `tmars.url`: Base URL of the TMars server
    ///   - `tmars.server_id`: TMars server identifier
    ///   - `tmars.polling_interval`: Seconds between TMars sync operations
    ///   - `matrix.user_id`: Matrix bot account ID (e.g., `@bot:example.com`)
    ///   - `matrix.password`: Matrix account password
    ///   - `matrix.passphrase`: E2EE recovery passphrase
    ///
    /// * `args` - Command line arguments containing:
    ///   - `data_path`: Directory path for storing Matrix session data and SQLite database
    ///
    /// # Returns
    ///
    /// * `Ok(Bot)` - Successfully initialized bot ready to start
    /// * `Err(anyhow::Error)` - Failed to create Matrix client (login failed, network error, etc.)
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - Matrix login fails (invalid credentials, network issues)
    /// - Session restoration fails (corrupted session file)
    /// - Cannot access the session storage path
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use miou::bot::Bot;
    /// # use miou::config::{Config, TMars, Matrix};
    /// # use miou::Args;
    /// # async fn example() -> Result<(), anyhow::Error> {
    /// # let args = Args {
    /// #     config: "config.toml".to_string(),
    /// #     data_path: "./data".to_string(),
    /// # };
    /// let config = Config {
    ///     tmars: TMars {
    ///         url: "https://tmars.example.com".to_string(),
    ///         server_id: "server1".to_string(),
    ///         polling_interval: 120,
    ///     },
    ///     matrix: Matrix {
    ///         user_id: "@bot:example.com".to_string(),
    ///         password: "secret".to_string(),
    ///         passphrase: "passphrase".to_string(),
    ///     },
    /// };
    ///
    /// let bot = Bot::new(config, args).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: Config, args: Args) -> Result<Self, anyhow::Error> {
        // Create tmars services
        let tmars_requester = TMarsRequester::new(&config.tmars.url, &config.tmars.server_id);
        let tmars_sync = Arc::new(Mutex::new(TMarsSync::new(tmars_requester)));

        // Create matrix client
        let matrix_client = Arc::new(
            MatrixClient::new(
                &UserCredentials {
                    user_id: config.matrix.user_id,
                    password: config.matrix.password,
                    passphrase: config.matrix.passphrase,
                },
                &get_path(&args.data_path, "session"),
            )
            .await?,
        );

        let alert_controller = Arc::new(Mutex::new(
            AlertController::new(get_path(&args.data_path, "alerts")).await,
        ));

        let commander = Arc::new(Commander::new());

        Ok(Bot {
            matrix_client,
            tmars_sync,
            polling_interval: config.tmars.polling_interval,
            alert_controller,
            commander,
        })
    }

    /// Starts the bot and begins processing messages and synchronizing game state.
    ///
    /// This method consumes `self` and runs indefinitely, managing two concurrent tasks:
    ///
    /// 1. **TMars Sync Task** (background):
    ///    - Runs every `polling_interval` seconds
    ///    - Fetches current game states from TMars server
    ///    - Cleans up alerts for ended games
    ///    - Identifies which players need notifications
    ///    - Spawns delayed notification tasks
    ///
    /// 2. **Matrix Sync Task** (main):
    ///    - Listens for new messages in Matrix rooms
    ///    - Parses messages into bot commands
    ///    - Executes commands and updates alert state
    ///    - Sends responses back to users
    ///
    /// # Lifecycle
    ///
    /// This method runs forever and only terminates if:
    /// - The process receives a termination signal (SIGINT, SIGTERM)
    /// - The Matrix sync encounters an unrecoverable error (panics)
    ///
    /// # Panics
    ///
    /// Panics if the Matrix sync loop fails to start or encounters an unrecoverable error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use miou::bot::Bot;
    /// # use miou::config::{Config, TMars, Matrix};
    /// # use miou::Args;
    /// # async fn example() -> Result<(), anyhow::Error> {
    /// # let config = Config {
    /// #     tmars: TMars {
    /// #         url: "https://tmars.example.com".to_string(),
    /// #         server_id: "server1".to_string(),
    /// #         polling_interval: 120,
    /// #     },
    /// #     matrix: Matrix {
    /// #         user_id: "@bot:example.com".to_string(),
    /// #         password: "secret".to_string(),
    /// #         passphrase: "passphrase".to_string(),
    /// #     },
    /// # };
    /// # let args = Args {
    /// #     config: "config.toml".to_string(),
    /// #     data_path: "./data".to_string(),
    /// # };
    /// let bot = Bot::new(config, args).await?;
    /// bot.start().await; // Runs until process termination
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(self) {
        let matrix_client_for_spawn = Arc::clone(&self.matrix_client);
        let tmars_sync = Arc::clone(&self.tmars_sync);
        let alert_controller = Arc::clone(&self.alert_controller);
        let polling_interval = self.polling_interval;

        self.alert_controller.lock().await.start_persistence_task();

        // Start tmars sync in a separate task to not block matrix sync
        self.start_tmars_sync_task(
            matrix_client_for_spawn,
            tmars_sync,
            Arc::clone(&alert_controller),
            polling_interval,
        );

        // Clone references for the message handler
        let matrix_client_for_handler = Arc::clone(&self.matrix_client);
        let tmars_sync_ref = Arc::clone(&self.tmars_sync);
        let commander = Arc::clone(&self.commander);

        // Create message handler closure
        let on_message =
            move |body: String, room_id: String, sender_id: String, event_id: String| {
                let ctx = MessageContext {
                    body,
                    room_id,
                    sender_id,
                    event_id,
                    matrix_client: Arc::clone(&matrix_client_for_handler),
                    tmars_sync: Arc::clone(&tmars_sync_ref),
                    commander: Arc::clone(&commander),
                    alert_controller: Arc::clone(&alert_controller),
                };
                Self::handle_matrix_message(ctx)
            };

        // Start matrix sync
        self.matrix_client.sync(on_message).await.unwrap();
    }

    /// Starts the TMars synchronization task in the background.
    ///
    /// This task periodically polls the TMars server for game state updates,
    /// manages alerts, and triggers notifications. The task runs independently
    /// and will continue until the process is terminated.
    ///
    /// # Arguments
    ///
    /// * `matrix_client` - Thread-safe reference to the Matrix client for sending notifications
    /// * `tmars_sync` - Thread-safe reference to the TMars synchronization service
    /// * `alert_controller` - Thread-safe reference to the alert controller
    /// * `polling_interval` - Number of seconds between sync operations
    ///
    /// # Behavior
    ///
    /// On each sync cycle:
    /// 1. Fetches current game states from the TMars server
    /// 2. Updates the alert controller with current game state
    /// 3. The controller identifies which players need notifications
    /// 4. Spawns delayed notification tasks for each alert
    /// 5. Sends Matrix mentions to users when delays expire
    ///
    /// # Note
    ///
    /// This method spawns a background task and returns immediately. The spawned
    /// task will run indefinitely until the process is terminated.
    fn start_tmars_sync_task(
        &self,
        matrix_client: Arc<MatrixClient>,
        tmars_sync: Arc<Mutex<TMarsSync<TMarsRequester>>>,
        alert_controller: Arc<Mutex<AlertController>>,
        polling_interval: u64,
    ) {
        tokio::spawn(async move {
            info!(
                "syncing with tmars server every {} seconds",
                polling_interval
            );
            let mut interval = time::interval(Duration::from_secs(polling_interval));

            loop {
                interval.tick().await;
                tmars_sync.lock().await.sync().await;

                let games_map = tmars_sync.lock().await.get_games();

                let on_alert_to_fire = {
                    let matrix_client = Arc::clone(&matrix_client);
                    move |alert: Alert| {
                        let matrix_client = Arc::clone(&matrix_client);
                        tokio::spawn(async move {
                            let _ = matrix_client
                                .send_mention(
                                    &alert.room_id,
                                    &Commander::get_player_turn_message(alert.player_url.clone()),
                                    &alert.user_id,
                                )
                                .await;
                        });
                    }
                };
                alert_controller
                    .lock()
                    .await
                    .update_alerts(&games_map, on_alert_to_fire)
                    .await;
            }
        });
    }

    /// Handles an incoming Matrix message and processes it as a command.
    ///
    /// This method implements the complete command processing flow:
    /// 1. Parse the message body to identify the command
    /// 2. Silently ignore if not a command or for a different bot
    /// 3. Send error response if command syntax is invalid
    /// 4. Create execution context with current game/alert state
    /// 5. Execute the command and get result
    /// 6. Update alerts via the alert controller (add/remove as needed)
    /// 7. Send success response to user
    ///
    /// # Arguments
    ///
    /// * `ctx` - The message context containing:
    ///   - `body`: The message text to parse
    ///   - `room_id`, `sender_id`, `event_id`: Matrix message metadata
    ///   - `matrix_client`: For sending responses
    ///   - `tmars_sync`: For accessing current game state
    ///   - `alert_controller`: For managing alerts
    ///   - `commander`: For parsing and executing commands
    ///
    /// # Behavior
    ///
    /// This method spawns a new async task to handle the message, allowing the Matrix
    /// sync loop to continue processing other messages without blocking.
    ///
    /// Commands that modify alerts (e.g., `register`, `unregister`) will update the
    /// alert controller's internal state, which is then persisted automatically by
    /// the controller's background persistence task.
    fn handle_matrix_message(ctx: MessageContext) {
        tokio::spawn(async move {
            // Parse body to extract command
            let result = match ctx.commander.parse(&ctx.body) {
                Ok(result) => result,
                Err(e) => match e {
                    // Return silently if the command is not for the bot
                    CommandParseError::NotForBot => return,
                    // Send error message if the command is invalid
                    CommandParseError::InvalidCommand(message) => {
                        ctx.matrix_client
                            .send_reply(&ctx.room_id, &ctx.sender_id, &ctx.event_id, &message)
                            .await;
                        return;
                    }
                },
            };

            let command_context = CommandContext {
                room_id: ctx.room_id.clone(),
                user_id: ctx.sender_id.clone(),
                games_map: ctx.tmars_sync.lock().await.get_games(),
                alerts_map: ctx.alert_controller.lock().await.get_alerts_map().await,
            };

            // Parse command with context
            let command_result = match ctx.commander.parse_command(&result, &command_context).await
            {
                Some(command_result) => command_result,
                None => return,
            };

            // Update alerts map based on command result
            if let Some((game_id, alert)) = command_result.alert_to_add {
                ctx.alert_controller
                    .lock()
                    .await
                    .add_alert(&game_id, &alert)
                    .await;
            }
            if let Some((game_id, room_id, user_id)) = command_result.alerts_to_remove {
                ctx.alert_controller
                    .lock()
                    .await
                    .remove_alerts(&game_id, &room_id, &user_id)
                    .await;
            }

            // Send response back to matrix room
            ctx.matrix_client
                .send_reply(
                    &ctx.room_id,
                    &ctx.sender_id,
                    &ctx.event_id,
                    &command_result.response,
                )
                .await;
        });
    }
}
