//! # Watchtower Subscriber
//!
//! Real-time Solana program event subscriber that connects to WebSocket endpoints
//! and Geyser plugins to monitor on-chain program activity.
//!
//! This module provides:
//! - WebSocket client for Solana RPC connections
//! - Event filtering and deserialization
//! - Program-specific event extraction
//! - Configurable subscription management

pub mod client;
pub mod config;
pub mod error;
pub mod events;
pub mod filters;

pub use client::*;
pub use config::*;
pub use error::*;
pub use events::*;
pub use filters::*;
