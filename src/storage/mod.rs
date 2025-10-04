//! Storage backends for metric persistence
//!
//! This module provides a trait-based abstraction for storing metrics
//! to various backends (SQLite, PostgreSQL, Parquet, etc.).
//!
//! ## Design
//!
//! - **Trait-based**: `StorageBackend` trait allows swapping implementations
//! - **Async**: All operations are async for compatibility with Tokio actors
//! - **Batch-oriented**: Optimized for batch writes to maximize throughput
//!
//! ## Backends
//!
//! - **SQLite** (default): Embedded database, good for <100 servers
//! - **PostgreSQL** (future): Production-grade with TimescaleDB support
//! - **Parquet** (future): Columnar storage for archival
//! - **In-Memory** (fallback): No persistence, for testing or backward compat
//!
//! ## Usage
//!
//! ```no_run
//! use server_monitoring::storage::{StorageBackend, sqlite::SqliteBackend};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let backend = SqliteBackend::new("./metrics.db").await?;
//!     // Use with StorageActor
//!     Ok(())
//! }
//! ```

pub mod backend;
pub mod error;
pub mod schema;
pub mod sqlite;

pub use backend::StorageBackend;
pub use error::{StorageError, StorageResult};
pub use schema::{MetricRow, MetricType};
