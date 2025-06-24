//! Error types for the notifier module.

use thiserror::Error;

/// Errors that can occur in the notifier module.
#[derive(Error, Debug)]
pub enum NotifierError {
    /// Email sending error
    #[error("Email sending failed: {0}")]
    Email(#[from] lettre::error::Error),

    /// Email address parsing error
    #[error("Email address parsing failed: {0}")]
    EmailAddress(#[from] lettre::address::AddressError),

    /// HTTP request error
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// Template rendering error
    #[error("Template rendering failed: {0}")]
    Template(#[from] tera::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded for channel: {channel}")]
    RateLimit { channel: String },

    /// Channel not configured
    #[error("Channel not configured: {channel}")]
    ChannelNotConfigured { channel: String },

    /// Message formatting error
    #[error("Message formatting error: {0}")]
    MessageFormat(String),

    /// Authentication error
    #[error("Authentication failed for {channel}: {reason}")]
    Authentication { channel: String, reason: String },

    /// Network timeout
    #[error("Network timeout for {channel} after {seconds} seconds")]
    Timeout { channel: String, seconds: u64 },

    /// Generic error
    #[error("Notifier error: {0}")]
    Generic(String),
}

/// Result type for notifier operations.
pub type NotifierResult<T> = Result<T, NotifierError>; 