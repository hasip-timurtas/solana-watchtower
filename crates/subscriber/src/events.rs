//! Event structures for Solana program monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use std::collections::HashMap;

/// A monitored program event that can trigger rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramEvent {
    /// Unique event identifier
    pub id: String,

    /// Program that generated this event
    pub program_id: Pubkey,

    /// Program name (from config)
    pub program_name: String,

    /// Event type
    pub event_type: EventType,

    /// Timestamp when the event occurred
    pub timestamp: DateTime<Utc>,

    /// Solana slot number
    pub slot: u64,

    /// Block time (if available)
    pub block_time: Option<i64>,

    /// Transaction signature (if applicable)
    pub signature: Option<Signature>,

    /// Event-specific data
    pub data: EventData,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Types of events that can be monitored.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventType {
    /// Transaction executed by the program
    Transaction,

    /// Account state changed
    AccountChange,

    /// Program log emitted
    LogEntry,

    /// Instruction executed
    Instruction,

    /// Token transfer (for token programs)
    TokenTransfer,

    /// Custom event type
    Custom { name: String },
}

/// Event-specific data payload.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "data_type")]
pub enum EventData {
    /// Transaction data
    Transaction {
        /// Transaction signature for reference
        signature: Signature,
        /// Success status
        success: bool,
        /// Compute units consumed
        compute_units: Option<u64>,
        /// Fee paid
        fee: u64,
    },

    /// Account change data
    AccountChange {
        /// Account public key
        account: Pubkey,
        /// Account balance before
        balance_before: Option<u64>,
        /// Account balance after
        balance_after: Option<u64>,
        /// Data size change
        data_size_change: i64,
        /// Owner program
        owner: Pubkey,
    },

    /// Log entry data
    LogEntry {
        /// Log message
        message: String,
        /// Log level (if parseable)
        level: Option<LogLevel>,
        /// Instruction context
        instruction_index: Option<usize>,
    },

    /// Instruction execution data
    Instruction {
        /// Instruction index in transaction
        index: usize,
        /// Instruction data
        data: Vec<u8>,
        /// Accounts involved
        accounts: Vec<Pubkey>,
        /// Execution success
        success: bool,
    },

    /// Token transfer data
    TokenTransfer {
        /// Source account
        from: Pubkey,
        /// Destination account
        to: Pubkey,
        /// Amount transferred
        amount: u64,
        /// Token mint
        mint: Pubkey,
        /// Decimals for display
        decimals: u8,
    },

    /// Custom event data
    Custom {
        /// Event name
        name: String,
        /// Arbitrary data
        data: serde_json::Value,
    },
}

impl Clone for EventData {
    fn clone(&self) -> Self {
        match self {
            EventData::Transaction {
                signature,
                success,
                compute_units,
                fee,
            } => EventData::Transaction {
                signature: *signature,
                success: *success,
                compute_units: *compute_units,
                fee: *fee,
            },
            EventData::AccountChange {
                account,
                balance_before,
                balance_after,
                data_size_change,
                owner,
            } => EventData::AccountChange {
                account: *account,
                balance_before: *balance_before,
                balance_after: *balance_after,
                data_size_change: *data_size_change,
                owner: *owner,
            },
            EventData::LogEntry {
                message,
                level,
                instruction_index,
            } => EventData::LogEntry {
                message: message.clone(),
                level: level.clone(),
                instruction_index: *instruction_index,
            },
            EventData::Instruction {
                index,
                data,
                accounts,
                success,
            } => EventData::Instruction {
                index: *index,
                data: data.clone(),
                accounts: accounts.clone(),
                success: *success,
            },
            EventData::TokenTransfer {
                from,
                to,
                amount,
                mint,
                decimals,
            } => EventData::TokenTransfer {
                from: *from,
                to: *to,
                amount: *amount,
                mint: *mint,
                decimals: *decimals,
            },
            EventData::Custom { name, data } => EventData::Custom {
                name: name.clone(),
                data: data.clone(),
            },
        }
    }
}

/// Log level for program logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl ProgramEvent {
    /// Create a new program event.
    pub fn new(
        program_id: Pubkey,
        program_name: String,
        event_type: EventType,
        data: EventData,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            program_id,
            program_name,
            event_type,
            timestamp: Utc::now(),
            slot: 0, // Will be set by subscriber
            block_time: None,
            signature: None,
            data,
            metadata: HashMap::new(),
        }
    }

    /// Add metadata to the event.
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set the slot number.
    pub fn with_slot(mut self, slot: u64) -> Self {
        self.slot = slot;
        self
    }

    /// Set the block time.
    pub fn with_block_time(mut self, block_time: Option<i64>) -> Self {
        self.block_time = block_time;
        self
    }

    /// Set the transaction signature.
    pub fn with_signature(mut self, signature: Option<Signature>) -> Self {
        self.signature = signature;
        self
    }

    /// Check if this is a transaction event.
    pub fn is_transaction(&self) -> bool {
        matches!(self.event_type, EventType::Transaction)
    }

    /// Check if this is an account change event.
    pub fn is_account_change(&self) -> bool {
        matches!(self.event_type, EventType::AccountChange)
    }

    /// Check if this is a log entry event.
    pub fn is_log_entry(&self) -> bool {
        matches!(self.event_type, EventType::LogEntry)
    }

    /// Get transaction signature if this is a transaction event.
    pub fn transaction_signature(&self) -> Option<&Signature> {
        match &self.data {
            EventData::Transaction { signature, .. } => Some(signature),
            _ => None,
        }
    }

    /// Get the transaction success status.
    pub fn is_successful(&self) -> Option<bool> {
        match &self.data {
            EventData::Transaction { success, .. } => Some(*success),
            EventData::Instruction { success, .. } => Some(*success),
            _ => None,
        }
    }
}

impl EventType {
    /// Get the string representation of the event type.
    pub fn as_str(&self) -> &str {
        match self {
            EventType::Transaction => "transaction",
            EventType::AccountChange => "account_change",
            EventType::LogEntry => "log_entry",
            EventType::Instruction => "instruction",
            EventType::TokenTransfer => "token_transfer",
            EventType::Custom { name } => name,
        }
    }
}

impl LogLevel {
    /// Parse log level from string.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "error" | "err" => Some(LogLevel::Error),
            "warn" | "warning" => Some(LogLevel::Warn),
            "info" => Some(LogLevel::Info),
            "debug" => Some(LogLevel::Debug),
            "trace" => Some(LogLevel::Trace),
            _ => None,
        }
    }
}
