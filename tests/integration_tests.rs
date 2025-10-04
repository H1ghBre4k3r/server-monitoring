//! Integration tests for the actor-based monitoring system

#[path = "integration/helpers.rs"]
mod helpers;

#[path = "integration/actor_pipeline.rs"]
mod actor_pipeline;

#[path = "integration/failure_scenarios.rs"]
mod failure_scenarios;

#[path = "integration/concurrency.rs"]
mod concurrency;

#[cfg(feature = "storage-sqlite")]
#[path = "integration/storage_persistence.rs"]
mod storage_persistence;
