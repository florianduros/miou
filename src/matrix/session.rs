use std::{fs::exists, path::PathBuf};

use tokio::fs;

use log::{debug, trace};
use matrix_sdk::authentication::matrix;
use serde::{Deserialize, Serialize};

/// Internal session data structure.
///
/// Contains the Matrix user session and optional sync token.
/// This is serialized to JSON and persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    /// The Matrix user session containing authentication credentials.
    user_session: matrix::MatrixSession,

    /// The latest sync token for resuming sync operations.
    ///
    /// Omitted from serialization when `None` to keep the file clean.
    #[serde(skip_serializing_if = "Option::is_none")]
    sync_token: Option<String>,
}

/// Matrix session manager.
///
/// Manages Matrix user sessions and persists authentication state to disk.
/// The session data is stored in a JSON file, and the SQLite database path
/// is tracked for use by the Matrix SDK.
///
/// # File Structure
///
/// The session directory contains:
/// - `session`: JSON file with user authentication and sync token
/// - `sqlite`: SQLite database for Matrix SDK state
///
/// # Examples
///
/// ```no_run
/// use miou::matrix::session::MatrixSession;
///
/// # async fn example() -> Result<(), anyhow::Error> {
/// let matrix_session = MatrixSession::new("path/to/session/dir".to_string()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MatrixSession {
    /// The user session if it exists.
    session: Option<Session>,
    /// Path to the sqlite database. Value is `dir_path/sqlite`
    sqlite_path: String,
    /// Path to the session file. Value is `dir_path/session`
    session_path: String,
}

impl MatrixSession {
    /// Create a new Matrix session manager.
    ///
    /// Attempts to load an existing session from the session file if it exists.
    /// If no session file is found, creates a new instance without a session.
    ///
    /// # Arguments
    ///
    /// * `dir_path` - The directory path to store the session and SQLite database.
    ///
    /// # Returns
    ///
    /// A new `MatrixSession` instance, or an error if path operations fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use miou::matrix::session::MatrixSession;
    /// # async fn example() -> Result<(), anyhow::Error> {
    /// let matrix_session = MatrixSession::new("path/to/session/dir".to_string()).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(dir_path: &str) -> Result<MatrixSession, anyhow::Error> {
        debug!("read session at {}", dir_path);

        let sqlite_path_buf: PathBuf = [dir_path, "sqlite"].iter().collect();
        let sqlite_path = sqlite_path_buf.to_str().unwrap().to_owned();
        debug!("sql path {}", sqlite_path);

        let session_path_buf: PathBuf = [dir_path, "session"].iter().collect();
        let session_path = session_path_buf.to_str().unwrap().to_owned();
        debug!("session path {}", session_path);

        let session = MatrixSession::get_session(&session_path).await.ok();
        debug!("found user session {:?}", session);

        Ok(MatrixSession {
            session,
            sqlite_path,
            session_path,
        })
    }

    /// Load the session from disk.
    ///
    /// # Arguments
    ///
    /// * `session_path` - The path to the session JSON file.
    ///
    /// # Returns
    ///
    /// The deserialized `Session`, or an error if the file doesn't exist or is invalid.
    async fn get_session(session_path: &str) -> Result<Session, anyhow::Error> {
        if !exists(session_path).unwrap_or_default() {
            return Err(anyhow::anyhow!("session file does not exist"));
        }

        let session_data = fs::read_to_string(session_path).await?;
        let session: Session = serde_json::from_str(&session_data).map_err(anyhow::Error::new)?;
        Ok(session)
    }

    /// Checks if a session is currently loaded.
    ///
    /// Returns `true` if a session file was found and loaded during initialization.
    pub fn has_session(&self) -> bool {
        self.session.is_some()
    }

    /// Returns the path to the SQLite database.
    ///
    /// The database is used by the Matrix SDK to store encryption keys and state.
    pub fn get_sqlite_path(&self) -> String {
        self.sqlite_path.clone()
    }

    /// Returns the user session if one is loaded.
    ///
    /// The user session contains authentication credentials and metadata.
    pub fn get_user_session(&self) -> Option<&matrix::MatrixSession> {
        self.session.as_ref().map(|s| &s.user_session)
    }

    /// Returns the sync token if one is stored.
    ///
    /// The sync token is used to resume synchronization from the last position.
    pub fn get_sync_token(&self) -> Option<String> {
        self.session.as_ref().and_then(|s| s.sync_token.clone())
    }

    /// Persists the sync token to disk.
    ///
    /// Updates the session file with the new sync token while preserving
    /// the user session data.
    ///
    /// # Arguments
    ///
    /// * `sync_token` - The sync token to persist
    ///
    /// # Errors
    ///
    /// Returns an error if the session file cannot be read, parsed, or written.
    pub async fn persist_sync_token(&self, sync_token: String) -> anyhow::Result<()> {
        trace!("persist sync token {}", sync_token);

        let serialized_session = fs::read_to_string(&self.session_path).await?;
        let mut full_session: Session = serde_json::from_str(&serialized_session)?;

        full_session.sync_token = Some(sync_token);
        let serialized_session = serde_json::to_string(&full_session)?;
        fs::write(&self.session_path, serialized_session).await?;

        trace!("sync token persisted");
        Ok(())
    }

    /// Persists the user session to disk.
    ///
    /// Creates a new session file with the provided user session data.
    /// The sync token is not included when creating a new session.
    ///
    /// # Arguments
    ///
    /// * `user_session` - The Matrix user session to persist
    ///
    /// # Errors
    ///
    /// Returns an error if the session file cannot be written.
    pub async fn persist_user_session(
        &self,
        user_session: &matrix::MatrixSession,
    ) -> anyhow::Result<()> {
        trace!("persist user session");

        let session = Session {
            user_session: user_session.clone(),
            sync_token: None,
        };

        let serialized_session = serde_json::to_string(&session)?;
        fs::write(&self.session_path, serialized_session).await?;

        trace!("user session persisted");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::{
        SessionMeta, SessionTokens, authentication::matrix::MatrixSession as SdkMatrixSession,
    };
    use tempfile::TempDir;
    use tokio::fs;

    // Helper function to create a mock MatrixSession
    fn create_mock_matrix_session() -> SdkMatrixSession {
        let session_meta = SessionMeta {
            user_id: "@test:example.com".try_into().unwrap(),
            device_id: "DEVICEID".into(),
        };

        let tokens = SessionTokens {
            access_token: "access_token".to_string(),
            refresh_token: Some("refresh_token".to_string()),
        };

        SdkMatrixSession {
            meta: session_meta,
            tokens,
        }
    }

    // Helper function to create a valid session JSON
    fn create_session_json() -> String {
        let mock_session = create_mock_matrix_session();
        let session = Session {
            user_session: mock_session,
            sync_token: Some("sync_token_123".to_string()),
        };
        serde_json::to_string(&session).unwrap()
    }

    #[tokio::test]
    async fn test_create_matrix_session_with_no_existing_session() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();

        let matrix_session = MatrixSession::new(&dir_path).await.unwrap();

        assert!(!matrix_session.has_session());
        assert_eq!(
            matrix_session.get_sqlite_path(),
            format!("{}/sqlite", dir_path)
        );
        assert!(matrix_session.get_user_session().is_none());
        assert!(matrix_session.get_sync_token().is_none());
    }

    #[tokio::test]
    async fn test_create_matrix_session_with_existing_session() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();
        let session_path = format!("{}/session", dir_path);

        // Create a session file
        let session_json = create_session_json();
        fs::write(&session_path, session_json).await.unwrap();

        let matrix_session = MatrixSession::new(&dir_path).await.unwrap();

        assert!(matrix_session.has_session());
        assert_eq!(
            matrix_session.get_sqlite_path(),
            format!("{}/sqlite", dir_path)
        );
        assert!(matrix_session.get_user_session().is_some());
        assert_eq!(
            matrix_session.get_sync_token(),
            Some("sync_token_123".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_session_file_does_not_exist() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = format!("{}/nonexistent_session", temp_dir.path().to_string_lossy());

        let result = MatrixSession::get_session(&session_path).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("session file does not exist")
        );
    }

    #[tokio::test]
    async fn test_get_session_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = format!("{}/invalid_session", temp_dir.path().to_string_lossy());

        // Write invalid JSON
        fs::write(&session_path, "invalid json").await.unwrap();

        let result = MatrixSession::get_session(&session_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_session_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = format!("{}/valid_session", temp_dir.path().to_string_lossy());

        // Write valid session JSON
        let session_json = create_session_json();
        fs::write(&session_path, session_json).await.unwrap();

        let result = MatrixSession::get_session(&session_path).await;
        assert!(result.is_ok());

        let session = result.unwrap();
        assert_eq!(session.sync_token, Some("sync_token_123".to_string()));
    }

    #[tokio::test]
    async fn test_has_session() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();

        // Test without session
        let matrix_session = MatrixSession::new(&dir_path).await.unwrap();
        assert!(!matrix_session.has_session());

        // Create session file and test again
        let session_path = format!("{}/session", dir_path);
        let session_json = create_session_json();
        fs::write(&session_path, session_json).await.unwrap();

        let matrix_session_with_session = MatrixSession::new(&dir_path).await.unwrap();
        assert!(matrix_session_with_session.has_session());
    }

    #[tokio::test]
    async fn test_get_sqlite_path() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();

        let matrix_session = MatrixSession::new(&dir_path).await.unwrap();
        let expected_path = format!("{}/sqlite", dir_path);

        assert_eq!(matrix_session.get_sqlite_path(), expected_path);
    }

    #[tokio::test]
    async fn test_get_user_session() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();

        // Test without session
        let matrix_session = MatrixSession::new(&dir_path).await.unwrap();
        assert!(matrix_session.get_user_session().is_none());

        // Create session file and test with session
        let session_path = format!("{}/session", dir_path);
        let session_json = create_session_json();
        fs::write(&session_path, session_json).await.unwrap();

        let matrix_session_with_session = MatrixSession::new(&dir_path).await.unwrap();
        let user_session = matrix_session_with_session.get_user_session();
        assert!(user_session.is_some());
        assert_eq!(
            user_session.unwrap().meta.user_id.to_string(),
            "@test:example.com"
        );
    }

    #[tokio::test]
    async fn test_get_sync_token() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();

        // Test without session
        let matrix_session = MatrixSession::new(&dir_path).await.unwrap();
        assert!(matrix_session.get_sync_token().is_none());

        // Create session file and test with session
        let session_path = format!("{}/session", dir_path);
        let session_json = create_session_json();
        fs::write(&session_path, session_json).await.unwrap();

        let matrix_session_with_session = MatrixSession::new(&dir_path).await.unwrap();
        assert_eq!(
            matrix_session_with_session.get_sync_token(),
            Some("sync_token_123".to_string())
        );
    }

    #[tokio::test]
    async fn test_persist_user_session() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();
        let session_path = format!("{}/session", dir_path);

        let matrix_session = MatrixSession::new(&dir_path).await.unwrap();
        let mock_user_session = create_mock_matrix_session();

        // Persist the user session
        let result = matrix_session
            .persist_user_session(&mock_user_session)
            .await;
        assert!(result.is_ok());

        // Verify the session was written to disk
        assert!(fs::metadata(&session_path).await.is_ok());

        // Read and verify the content
        let session_content = fs::read_to_string(&session_path).await.unwrap();
        let session: Session = serde_json::from_str(&session_content).unwrap();
        assert_eq!(
            session.user_session.meta.user_id.to_string(),
            "@test:example.com"
        );
        assert!(session.sync_token.is_none());
    }

    #[tokio::test]
    async fn test_persist_sync_token() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().to_string_lossy().to_string();
        let session_path = format!("{}/session", dir_path);

        // First create a session with a user session
        let matrix_session = MatrixSession::new(&dir_path).await.unwrap();
        let mock_user_session = create_mock_matrix_session();
        matrix_session
            .persist_user_session(&mock_user_session)
            .await
            .unwrap();

        // Now persist a sync token
        let sync_token = "new_sync_token_456".to_string();
        let result = matrix_session.persist_sync_token(sync_token.clone()).await;
        assert!(result.is_ok());

        // Verify the sync token was updated
        let session_content = fs::read_to_string(&session_path).await.unwrap();
        let session: Session = serde_json::from_str(&session_content).unwrap();
        assert_eq!(session.sync_token, Some(sync_token));
    }

    #[tokio::test]
    async fn test_session_serialization_deserialization() {
        let mock_session = create_mock_matrix_session();
        let session = Session {
            user_session: mock_session,
            sync_token: Some("test_token".to_string()),
        };

        // Test serialization
        let serialized = serde_json::to_string(&session).unwrap();
        assert!(serialized.contains("test_token"));
        assert!(serialized.contains("@test:example.com"));

        // Test deserialization
        let deserialized: Session = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.sync_token, Some("test_token".to_string()));
        assert_eq!(
            deserialized.user_session.meta.user_id.to_string(),
            "@test:example.com"
        );
    }

    #[tokio::test]
    async fn test_session_serialization_without_sync_token() {
        let mock_session = create_mock_matrix_session();
        let session = Session {
            user_session: mock_session,
            sync_token: None,
        };

        // Test serialization - sync_token should be omitted when None
        let serialized = serde_json::to_string(&session).unwrap();
        assert!(!serialized.contains("sync_token"));
        assert!(serialized.contains("@test:example.com"));

        // Test deserialization
        let deserialized: Session = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.sync_token, None);
    }
}
