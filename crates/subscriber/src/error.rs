//! Error types for the subscriber module.

use thiserror::Error;

/// Errors that can occur in the subscriber module.
#[derive(Error, Debug)]
pub enum SubscriberError {
    /// WebSocket connection error
    #[error("WebSocket connection failed: {0}")]
    WebSocketConnection(#[from] tokio_tungstenite::tungstenite::Error),

    /// URL parsing error
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Solana client error
    #[error("Solana client error: {0}")]
    SolanaClient(#[from] solana_client::client_error::ClientError),

    /// Invalid subscription configuration
    #[error("Invalid subscription config: {0}")]
    InvalidConfig(String),

    /// Connection timeout
    #[error("Connection timeout after {seconds} seconds")]
    Timeout { seconds: u64 },

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Subscription failed
    #[error("Failed to subscribe to {subscription_type}: {reason}")]
    SubscriptionFailed {
        subscription_type: String,
        reason: String,
    },

    /// Event processing error
    #[error("Failed to process event: {0}")]
    EventProcessing(String),

    /// Generic error
    #[error("Subscriber error: {0}")]
    Generic(String),
}

/// Result type for subscriber operations.
pub type SubscriberResult<T> = Result<T, SubscriberError>; 