//! Error types for storage operations

use std::fmt;

/// Result type alias for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Errors that can occur during storage operations
#[derive(Debug)]
pub enum StorageError {
    /// Database connection failed
    ConnectionFailed(String),

    /// Database query failed
    QueryFailed(String),

    /// Migration failed
    MigrationFailed(String),

    /// Invalid configuration
    InvalidConfig(String),

    /// Metric serialization/deserialization error
    SerializationError(String),

    /// Backend-specific error
    BackendError(String),

    /// I/O error (file access, etc.)
    IoError(std::io::Error),

    /// The backend is not healthy
    UnhealthyBackend(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::ConnectionFailed(msg) => {
                write!(f, "failed to connect to storage backend: {}", msg)
            }
            StorageError::QueryFailed(msg) => write!(f, "storage query failed: {}", msg),
            StorageError::MigrationFailed(msg) => write!(f, "database migration failed: {}", msg),
            StorageError::InvalidConfig(msg) => write!(f, "invalid storage configuration: {}", msg),
            StorageError::SerializationError(msg) => {
                write!(f, "metric serialization error: {}", msg)
            }
            StorageError::BackendError(msg) => write!(f, "storage backend error: {}", msg),
            StorageError::IoError(err) => write!(f, "I/O error: {}", err),
            StorageError::UnhealthyBackend(msg) => write!(f, "storage backend unhealthy: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err)
    }
}

// sqlx error conversion (will be used in sqlite.rs)
#[cfg(feature = "storage-sqlite")]
impl From<sqlx::Error> for StorageError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Io(io_err) => StorageError::IoError(io_err),
            sqlx::Error::RowNotFound => StorageError::QueryFailed("no rows found".to_string()),
            _ => StorageError::QueryFailed(err.to_string()),
        }
    }
}

#[cfg(feature = "storage-sqlite")]
impl From<sqlx::migrate::MigrateError> for StorageError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        StorageError::MigrationFailed(err.to_string())
    }
}
