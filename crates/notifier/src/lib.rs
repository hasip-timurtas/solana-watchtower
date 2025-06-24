//! # Watchtower Notifier
//!
//! Multi-channel notification system for Solana monitoring alerts.
//!
//! This module provides:
//! - Abstract notifier trait for multiple channels
//! - Email notifications via SMTP
//! - Telegram bot notifications
//! - Slack and Discord webhook support
//! - Rate limiting and alert batching

pub mod channels;
pub mod config;
pub mod error;
pub mod manager;
pub mod templates;

pub use channels::*;
pub use config::*;
pub use error::*;
pub use manager::*;
pub use templates::*; 