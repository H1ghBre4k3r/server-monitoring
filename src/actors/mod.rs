//! Actor-based monitoring system
//!
//! This module implements an actor-based architecture for the monitoring system.
//! Each actor runs as an independent async task communicating via Tokio channels.
//!
//! ## Architecture Overview
//!
//! ```text
//!                    ┌─────────────────┐
//!                    │   Hub (main)    │
//!                    └────────┬────────┘
//!                             │ spawns
//!                ┌────────────┼────────────┐
//!                │            │            │
//!        ┌───────▼───────┐    │    ┌───────▼───────┐
//!        │ Collector-1   │    │    │ Collector-N   │
//!        │ (Server A)    │    │    │ (Server N)    │
//!        └───────┬───────┘    │    └───────┬───────┘
//!                │            │            │
//!                └────────────┼────────────┘
//!                             │
//!                   ┌─────────▼──────────┐
//!                   │  Broadcast Channel │ (metrics)
//!                   │  (MPMC)            │
//!                   └──────────┬─────────┘
//!                              │ subscribe
//!              ┌───────────────┼───────────────┐
//!              │               │               │
//!      ┌───────▼───────┐  ┌────▼─────┐  ┌──────▼──────┐
//!      │ StorageActor  │  │AlertActor│  │  ApiActor   │
//!      └───────────────┘  └──────────┘  └─────────────┘
//! ```
//!
//! ## Actor Types
//!
//! - **MetricCollectorActor**: Polls agent endpoints at configured intervals
//! - **AlertActor**: Evaluates metrics against thresholds and sends alerts
//! - **StorageActor**: Persists metrics to database (Phase 2)
//!
//! ## Communication Patterns
//!
//! 1. **Commands**: Each actor has an mpsc command channel for control messages
//! 2. **Events**: Actors publish events to broadcast channels for fan-out
//! 3. **Request/Response**: oneshot channels for synchronous queries

pub mod alert;
pub mod collector;
pub mod messages;
pub mod service_monitor;
pub mod storage;
