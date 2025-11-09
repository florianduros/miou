//! Matrix client encryption setup and recovery.
//!
//! This module handles the complete encryption lifecycle for Matrix clients, including:
//! - Cross-signing key management
//! - Key backup and secret storage setup
//! - Secret recovery using passphrases
//! - Session creation and restoration
//!
//! # Overview
//!
//! Matrix end-to-end encryption requires several components to work together:
//! - **Cross-signing**: Validates device trust without manual verification
//! - **Key backup**: Stores encrypted message keys on the server for recovery
//! - **Secret storage**: Securely stores encryption secrets protected by a passphrase
//!
//! This module provides a high-level [`setup_client`] function that handles all these
//! components automatically, either creating a new session or restoring an existing one.

use anyhow::bail;
use log::{debug, error, info};
use matrix_sdk::{
    Client,
    encryption::{
        BackupDownloadStrategy, EncryptionSettings,
        recovery::{RecoveryError, RecoveryState},
    },
    ruma::{OwnedUserId, api::client::uiaa},
};

use crate::matrix::{UserCredentials, session::MatrixSession};

/// Bootstraps cross-signing for the Matrix client if not already configured.
///
/// Cross-signing allows users to verify their devices without manual verification,
/// by establishing a trust chain from their master signing key. This function:
/// 1. Attempts to bootstrap cross-signing without authentication
/// 2. If UIAA (User-Interactive Authentication API) is required, retries with password
/// 3. Skips setup if cross-signing is already configured
///
/// See <https://docs.rs/matrix-sdk/latest/matrix_sdk/encryption/struct.Encryption.html#method.bootstrap_cross_signing_if_needed>
///
/// # Arguments
///
/// * `client` - The Matrix client to bootstrap
/// * `user_credentials` - User credentials containing the user ID and password for UIAA
///
/// # Errors
///
/// Returns an error if:
/// - Cross-signing bootstrap fails after authentication
/// - The password authentication fails
///
/// # Examples
///
/// ```ignore
/// let credentials = UserCredentials {
///     user_id: "@user:example.com".to_string(),
///     password: "password".to_string(),
///     passphrase: "passphrase".to_string(),
/// };
/// bootstrap_cross_signing(&client, &credentials).await?;
/// ```
async fn bootstrap_cross_signing(
    client: &Client,
    UserCredentials {
        user_id,
        password,
        passphrase: _,
    }: &UserCredentials,
) -> Result<(), anyhow::Error> {
    debug!("setting up cross signing");

    if let Err(e) = client
        .encryption()
        .bootstrap_cross_signing_if_needed(None)
        .await
    {
        let response = e.as_uiaa_response().unwrap();
        let mut password = uiaa::Password::new(
            uiaa::UserIdentifier::UserIdOrLocalpart(user_id.to_owned()),
            password.to_owned(),
        );
        password.session = response.session.clone();

        // Note, on the failed attempt we can use `bootstrap_cross_signing` immediately, to
        // avoid checks.
        client
            .encryption()
            .bootstrap_cross_signing(Some(uiaa::AuthData::Password(password)))
            .await?;

        debug!("cross signing set up");
        return Ok(());
    }

    debug!("cross signing already set up");
    Ok(())
}

/// Enables key backup and secret storage for the Matrix client.
///
/// Key backup ensures encrypted message keys are stored on the server, allowing
/// recovery of message history on new devices. Secret storage protects critical
/// encryption secrets with a passphrase. This function:
/// 1. Enables recovery with the provided passphrase
/// 2. Gracefully handles the case where backup already exists
/// 3. Returns an error for other failure cases
///
/// See <https://docs.rs/matrix-sdk/latest/matrix_sdk/encryption/recovery/struct.Recovery.html#method.enable>
///
/// # Arguments
///
/// * `client` - The Matrix client to configure
/// * `user_credentials` - User credentials containing the passphrase for secret storage
///
/// # Errors
///
/// Returns an error if recovery enabling fails for reasons other than backup already existing.
///
/// # Examples
///
/// ```ignore
/// let credentials = UserCredentials {
///     user_id: "@user:example.com".to_string(),
///     password: "password".to_string(),
///     passphrase: "secure_passphrase".to_string(),
/// };
/// enable_recovery(&client, &credentials).await?;
/// ```
async fn enable_recovery(
    client: &Client,
    user_credentials: &UserCredentials,
) -> Result<(), anyhow::Error> {
    debug!("enabling recovery");

    let recovery = client.encryption().recovery();

    match recovery
        .enable()
        .with_passphrase(&user_credentials.passphrase)
        .await
    {
        Ok(_) => debug!("recovery enabled"),
        Err(e) => match e {
            RecoveryError::BackupExistsOnServer => {
                debug!("recovery already enabled");
            }
            _ => bail!("error enabling recovery: {:?}", e),
        },
    }

    Ok(())
}

/// Verifies that encryption is properly configured for the Matrix client.
///
/// This function performs critical validation checks to ensure:
/// 1. Recovery (key backup and secret storage) is enabled
/// 2. The current device is verified (part of the cross-signing trust chain)
///
/// These checks are essential before the client can safely participate in encrypted rooms.
///
/// # Arguments
///
/// * `client` - The Matrix client to validate
///
/// # Errors
///
/// Returns an error if:
/// - Recovery is not in the `Enabled` state
/// - The device is not verified (not trusted by cross-signing)
///
/// # Examples
///
/// ```ignore
/// // After setting up encryption
/// encryption_check(&client).await?;
/// // Client is now ready for encrypted messaging
/// ```
async fn encryption_check(client: &Client) -> Result<(), anyhow::Error> {
    let recovery = client.encryption().recovery();
    if recovery.state() != RecoveryState::Enabled {
        error!("recovery is not enabled after enabling it");
        return Err(anyhow::anyhow!("recovery is disabled after enabling it"));
    }

    // Client is logged in so we can get the device without catching an error
    let device = client.encryption().get_own_device().await?.unwrap();
    if !device.is_verified() {
        error!("device is not verified after setting up encryption");
        return Err(anyhow::anyhow!(
            "device is not verified after setting up encryption"
        ));
    }

    Ok(())
}

/// Creates a new Matrix client session with full encryption setup.
///
/// This function performs the complete initialization workflow for a new Matrix session:
/// 1. Creates a client with encryption settings (cross-signing and backups enabled)
/// 2. Configures SQLite storage with passphrase encryption
/// 3. Logs in to the Matrix server
/// 4. Bootstraps cross-signing if needed (may require password authentication)
/// 5. Enables key backup and secret storage with the passphrase
/// 6. Recovers all secrets from secret storage
/// 7. Validates encryption setup (recovery enabled, device verified)
/// 8. Persists the session to disk for future restoration
///
/// # Arguments
///
/// * `user_credentials` - The user credentials containing user ID, password, and passphrase
/// * `matrix_session` - The session manager to persist the authenticated session
///
/// # Returns
///
/// An authenticated and fully configured [`Client`] ready for encrypted messaging.
///
/// # Errors
///
/// Returns an error if any step fails:
/// - Client creation or login fails
/// - Cross-signing or recovery setup fails
/// - Secret recovery fails
/// - Encryption validation fails
/// - Session persistence fails
///
/// # Examples
///
/// ```ignore
/// use crate::matrix::{UserCredentials, session::MatrixSession};
///
/// let credentials = UserCredentials {
///     user_id: "@bot:example.com".to_string(),
///     password: "bot_password".to_string(),
///     passphrase: "secure_passphrase_for_secrets".to_string(),
/// };
/// let session = MatrixSession::new("./session".to_string()).await?;
/// let client = create_session(&credentials, &session).await?;
/// ```
async fn create_session(
    user_credentials: &UserCredentials,
    matrix_session: &MatrixSession,
) -> Result<Client, anyhow::Error> {
    // Enable key backup and cross signing by default
    let encryption_settings = EncryptionSettings {
        auto_enable_cross_signing: true,
        backup_download_strategy: BackupDownloadStrategy::default(),
        auto_enable_backups: true,
    };

    // Create the client
    let miou: OwnedUserId = user_credentials.user_id.clone().try_into()?;
    let client = Client::builder()
        .sqlite_store(
            matrix_session.get_sqlite_path(),
            Some(&user_credentials.passphrase),
        )
        .with_encryption_settings(encryption_settings)
        .server_name(miou.server_name())
        .build()
        .await?;

    debug!("matrix client created");

    // Log in
    client
        .matrix_auth()
        .login_username(miou, &user_credentials.password)
        .initial_device_display_name("miou bot")
        .send()
        .await?;

    // Bootstrap cross signing if needed
    // Error handling is done inside the function
    bootstrap_cross_signing(&client, user_credentials).await?;
    // Enable key backup and secret storage if needed
    // Error handling is done inside the function
    enable_recovery(&client, user_credentials).await?;

    // Recover all secrets using the passphrase
    debug!("trying to recover secrets");
    let recovery = client.encryption().recovery();
    recovery.recover(&user_credentials.passphrase).await?;
    debug!("secrets recovered");

    // Final encryption check
    encryption_check(&client).await?;

    // Persist the user session
    let user_session = client.matrix_auth().session().unwrap();
    if let Err(err) = matrix_session.persist_user_session(&user_session).await {
        error!("error persisting user session: {:?}", err);
        return Err(anyhow::anyhow!("error persisting user session: {:?}", err));
    }

    info!("matrix client setup complete");
    Ok(client)
}

/// Restores an existing Matrix client session from persisted storage.
///
/// This function restores a previously created session by:
/// 1. Creating a client with SQLite storage access (using passphrase)
/// 2. Restoring the authenticated session from disk
/// 3. Opening secret storage with the passphrase
/// 4. Importing all secrets (encryption keys, cross-signing keys, etc.)
/// 5. Validating encryption setup (recovery enabled, device verified)
///
/// This is faster than creating a new session as it avoids login and
/// potentially expensive key backup downloads.
///
/// # Arguments
///
/// * `user_credentials` - The user credentials containing the passphrase for decryption
/// * `matrix_session` - The session manager containing the persisted session data
///
/// # Returns
///
/// A restored and fully configured [`Client`] with access to all encryption keys.
///
/// # Errors
///
/// Returns an error if:
/// - Client creation fails
/// - Session restoration fails (invalid or expired session)
/// - Secret storage cannot be opened (wrong passphrase)
/// - Secret import fails
/// - Encryption validation fails
///
/// # Examples
///
/// ```ignore
/// // Assumes a session was previously created and persisted
/// let credentials = UserCredentials {
///     user_id: "@bot:example.com".to_string(),
///     password: "bot_password".to_string(),
///     passphrase: "secure_passphrase_for_secrets".to_string(),
/// };
/// let session = MatrixSession::new("./session".to_string()).await?;
/// let client = restore_session(&credentials, &session).await?;
/// ```
async fn restore_session(
    user_credentials: &UserCredentials,
    matrix_session: &MatrixSession,
) -> Result<Client, anyhow::Error> {
    info!("restoring matrix session from disk");

    let miou: OwnedUserId = user_credentials.user_id.clone().try_into()?;
    let client: Client = Client::builder()
        .server_name(miou.server_name())
        .sqlite_store(
            matrix_session.get_sqlite_path(),
            Some(&user_credentials.passphrase),
        )
        .build()
        .await?;

    // Restore the session
    client
        .restore_session(matrix_session.get_user_session().unwrap().clone())
        .await?;

    // Import secrets from secret storage
    let secret_store = client
        .encryption()
        .secret_storage()
        .open_secret_store(&user_credentials.passphrase)
        .await
        .unwrap();
    secret_store.import_secrets().await.unwrap();

    // Final encryption check
    encryption_check(&client).await?;

    info!("matrix session restored successfully");

    Ok(client)
}

/// Sets up a Matrix client with full encryption, cross-signing, and key backup.
///
/// This is the main entry point for obtaining a configured Matrix client. It automatically
/// determines whether to create a new session or restore an existing one:
///
/// **New Session (no persisted data):**
/// - Logs in to the Matrix server
/// - Bootstraps cross-signing keys
/// - Enables key backup and secret storage
/// - Recovers and validates all encryption secrets
/// - Persists session for future restoration
///
/// **Existing Session (persisted data found):**
/// - Restores the authenticated session from disk
/// - Imports all encryption secrets from secret storage
/// - Validates encryption setup
///
/// Both paths result in a fully configured client ready for encrypted messaging.
///
/// # Arguments
///
/// * `user_credentials` - User credentials with user ID, password, and passphrase
/// * `matrix_session` - Session manager for persisting and restoring session data
///
/// # Returns
///
/// A fully configured [`Client`] with:
/// - Active authenticated session
/// - Cross-signing enabled and device verified
/// - Key backup and secret storage configured
/// - All encryption keys available
///
/// # Errors
///
/// Returns an error if:
/// - Session creation fails (login, encryption setup, persistence)
/// - Session restoration fails (invalid session, wrong passphrase, missing secrets)
/// - Encryption validation fails in either path
///
/// # Examples
///
/// ```no_run
/// use miou::matrix::{UserCredentials, session::MatrixSession};
/// use miou::matrix::encryption::setup_client;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), anyhow::Error> {
/// let credentials = UserCredentials {
///     user_id: "@my_bot:matrix.org".to_string(),
///     password: "my_secure_password".to_string(),
///     passphrase: "my_very_secure_passphrase_for_encryption".to_string(),
/// };
///
/// let session = MatrixSession::new("./bot_session".to_string()).await?;
/// let client = setup_client(&credentials, &session).await?;
///
/// // Client is now ready for encrypted messaging
/// # Ok(())
/// # }
/// ```
pub async fn setup_client(
    user_credentials: &UserCredentials,
    matrix_session: &MatrixSession,
) -> Result<Client, anyhow::Error> {
    info!(
        "setting up matrix client for user {}",
        user_credentials.user_id
    );

    if matrix_session.has_session() {
        return restore_session(user_credentials, matrix_session).await;
    } else {
        return create_session(user_credentials, matrix_session).await;
    }
}
