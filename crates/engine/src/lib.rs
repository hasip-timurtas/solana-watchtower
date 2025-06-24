//! # Watchtower Engine
//!
//! Rule engine and metrics aggregator for Solana program monitoring.
//! 
//! This module provides:
//! - Rule trait and built-in security rules
//! - Metrics collection and aggregation
//! - Alert generation based on rule violations
//! - Sliding window analysis for time-based rules

pub mod metrics;
pub mod rules;
pub mod engine;
pub mod alerts;

pub use metrics::*;
pub use rules::*;
pub use engine::*;
pub use alerts::*; 