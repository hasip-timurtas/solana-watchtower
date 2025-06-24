//! Configuration structures for the subscriber module.

use serde::{Deserialize, Deserializer, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::Duration;
use url::Url;

// Custom deserializer for Pubkey from string
fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

/// Configuration for the subscriber module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriberConfig {
    /// Solana RPC HTTP URL
    pub rpc_url: Url,

    /// Solana WebSocket URL
    pub ws_url: Url,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Maximum reconnection attempts
    #[serde(default = "default_max_reconnects")]
    pub max_reconnect_attempts: u32,

    /// Reconnection delay in seconds
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_seconds: u64,

    /// Programs to monitor
    pub programs: Vec<ProgramConfig>,

    /// Subscription filters
    #[serde(default)]
    pub filters: SubscriptionFilters,
}

/// Configuration for a specific program to monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramConfig {
    /// Program public key
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub id: Pubkey,

    /// Human-readable name for the program
    pub name: String,

    /// Whether to monitor account changes
    #[serde(default = "default_true")]
    pub monitor_accounts: bool,

    /// Whether to monitor transactions
    #[serde(default = "default_true")]
    pub monitor_transactions: bool,

    /// Whether to monitor logs
    #[serde(default = "default_true")]
    pub monitor_logs: bool,

    /// Custom instruction filters (optional)
    pub instruction_filters: Option<Vec<String>>,
}

/// Subscription filter configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscriptionFilters {
    /// Include failed transactions
    #[serde(default)]
    pub include_failed: bool,

    /// Include vote transactions
    #[serde(default)]
    pub include_votes: bool,

    /// Maximum transactions per notification
    #[serde(default = "default_max_transactions")]
    pub max_transactions_per_notification: usize,

    /// Commitment level
    #[serde(default = "default_commitment")]
    pub commitment: String,
}

impl SubscriberConfig {
    /// Get connection timeout as Duration
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds)
    }

    /// Get reconnect delay as Duration
    pub fn reconnect_delay(&self) -> Duration {
        Duration::from_secs(self.reconnect_delay_seconds)
    }

    /// Validate the configuration
    pub fn validate(&self) -> crate::SubscriberResult<()> {
        if self.programs.is_empty() {
            return Err(crate::SubscriberError::InvalidConfig(
                "At least one program must be configured".to_string(),
            ));
        }

        if self.timeout_seconds == 0 {
            return Err(crate::SubscriberError::InvalidConfig(
                "Timeout must be greater than 0".to_string(),
            ));
        }

        for program in &self.programs {
            if program.name.is_empty() {
                return Err(crate::SubscriberError::InvalidConfig(format!(
                    "Program {} must have a name",
                    program.id
                )));
            }
        }

        Ok(())
    }
}

impl ProgramConfig {
    /// Check if any monitoring is enabled for this program
    pub fn has_monitoring_enabled(&self) -> bool {
        self.monitor_accounts || self.monitor_transactions || self.monitor_logs
    }
}

// Default value functions
fn default_timeout() -> u64 {
    30
}

fn default_max_reconnects() -> u32 {
    5
}

fn default_reconnect_delay() -> u64 {
    5
}

fn default_true() -> bool {
    true
}

fn default_max_transactions() -> usize {
    100
}

fn default_commitment() -> String {
    "confirmed".to_string()
}
