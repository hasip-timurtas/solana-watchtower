//! WebSocket client for real-time Solana program event monitoring.

use crate::{
    config::SubscriberConfig,
    error::{SubscriberError, SubscriberResult},
    events::{EventData, EventType, ProgramEvent},
    filters::{EventFilter, SubscriptionManager, SubscriptionType},
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// WebSocket client for subscribing to Solana program events.
pub struct SolanaWebSocketClient {
    /// Client configuration
    config: SubscriberConfig,
    
    /// Event filter
    filter: EventFilter,
    
    /// Subscription manager
    subscription_manager: SubscriptionManager,
    
    /// Event sender
    event_sender: broadcast::Sender<ProgramEvent>,
    
    /// Connection status
    is_connected: Arc<tokio::sync::RwLock<bool>>,
}

/// WebSocket message types from Solana RPC.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "method")]
enum WebSocketMessage {
    #[serde(rename = "accountNotification")]
    AccountNotification {
        params: AccountNotificationParams,
    },
    
    #[serde(rename = "programNotification")]
    ProgramNotification {
        params: ProgramNotificationParams,
    },
    
    #[serde(rename = "signatureNotification")]
    SignatureNotification {
        params: SignatureNotificationParams,
    },
    
    #[serde(rename = "logsNotification")]
    LogsNotification {
        params: LogsNotificationParams,
    },
    
    #[serde(rename = "slotNotification")]
    SlotNotification {
        params: SlotNotificationParams,
    },
    
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AccountNotificationParams {
    result: AccountNotificationResult,
    subscription: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AccountNotificationResult {
    context: NotificationContext,
    value: AccountInfo,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProgramNotificationParams {
    result: ProgramNotificationResult,
    subscription: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProgramNotificationResult {
    context: NotificationContext,
    value: ProgramAccountInfo,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SignatureNotificationParams {
    result: SignatureNotificationResult,
    subscription: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SignatureNotificationResult {
    context: NotificationContext,
    value: SignatureStatus,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct LogsNotificationParams {
    result: LogsNotificationResult,
    subscription: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct LogsNotificationResult {
    context: NotificationContext,
    value: LogsInfo,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SlotNotificationParams {
    result: SlotInfo,
    subscription: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct NotificationContext {
    slot: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AccountInfo {
    executable: bool,
    lamports: u64,
    owner: String,
    #[serde(rename = "rentEpoch")]
    rent_epoch: u64,
    data: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProgramAccountInfo {
    account: AccountInfo,
    pubkey: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SignatureStatus {
    err: Option<Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct LogsInfo {
    signature: String,
    err: Option<Value>,
    logs: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct SlotInfo {
    parent: u64,
    root: u64,
    slot: u64,
}

impl SolanaWebSocketClient {
    /// Create a new WebSocket client.
    pub fn new(config: SubscriberConfig) -> SubscriberResult<Self> {
        config.validate()?;
        
        let filter = EventFilter::new(
            config.programs.clone(),
            config.filters.include_failed,
            config.filters.include_votes,
        );
        
        let (event_sender, _) = broadcast::channel(1000);
        
        Ok(Self {
            config,
            filter,
            subscription_manager: SubscriptionManager::new(),
            event_sender,
            is_connected: Arc::new(tokio::sync::RwLock::new(false)),
        })
    }
    
    /// Start the WebSocket client and begin monitoring.
    pub async fn start(&mut self) -> SubscriberResult<broadcast::Receiver<ProgramEvent>> {
        info!("Starting Solana WebSocket client");
        
        let receiver = self.event_sender.subscribe();
        
        // Start connection task
        let config = self.config.clone();
        let sender = self.event_sender.clone();
        let is_connected = self.is_connected.clone();
        
        tokio::spawn(async move {
            Self::connection_task(config, sender, is_connected).await;
        });
        
        Ok(receiver)
    }
    
    /// Connection task that handles WebSocket connection and reconnection.
    async fn connection_task(
        config: SubscriberConfig,
        event_sender: broadcast::Sender<ProgramEvent>,
        is_connected: Arc<tokio::sync::RwLock<bool>>,
    ) {
        let mut reconnect_attempts = 0;
        
        loop {
            match Self::connect_and_subscribe(&config, &event_sender, &is_connected).await {
                Ok(_) => {
                    info!("WebSocket connection closed gracefully");
                    reconnect_attempts = 0;
                }
                Err(e) => {
                    error!("WebSocket connection error: {}", e);
                    
                    *is_connected.write().await = false;
                    
                    reconnect_attempts += 1;
                    if reconnect_attempts > config.max_reconnect_attempts {
                        error!("Max reconnection attempts reached, stopping client");
                        break;
                    }
                    
                    warn!(
                        "Reconnecting in {} seconds (attempt {}/{})",
                        config.reconnect_delay_seconds,
                        reconnect_attempts,
                        config.max_reconnect_attempts
                    );
                    
                    tokio::time::sleep(config.reconnect_delay()).await;
                }
            }
        }
    }
    
    /// Connect to WebSocket and handle subscriptions.
    async fn connect_and_subscribe(
        config: &SubscriberConfig,
        event_sender: &broadcast::Sender<ProgramEvent>,
        is_connected: &Arc<tokio::sync::RwLock<bool>>,
    ) -> SubscriberResult<()> {
        info!("Connecting to WebSocket: {}", config.ws_url);
        
        let (ws_stream, _) = connect_async(&config.ws_url).await?;
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        *is_connected.write().await = true;
        info!("WebSocket connected successfully");
        
        // Subscribe to programs
        for program in &config.programs {
            if program.monitor_accounts || program.monitor_transactions {
                let subscription_request = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "programSubscribe",
                    "params": [
                        program.id.to_string(),
                        {
                            "commitment": config.filters.commitment,
                            "encoding": "jsonParsed"
                        }
                    ]
                });
                
                let message = Message::Text(subscription_request.to_string());
                ws_sender.send(message).await?;
                
                info!("Subscribed to program: {} ({})", program.name, program.id);
            }
            
            if program.monitor_logs {
                let logs_request = json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "logsSubscribe",
                    "params": [
                        {
                            "mentions": [program.id.to_string()]
                        },
                        {
                            "commitment": config.filters.commitment
                        }
                    ]
                });
                
                let message = Message::Text(logs_request.to_string());
                ws_sender.send(message).await?;
                
                info!("Subscribed to logs for program: {} ({})", program.name, program.id);
            }
        }
        
        // Handle incoming messages
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Err(e) = Self::handle_message(&text, config, event_sender).await {
                        error!("Error handling message: {}", e);
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        *is_connected.write().await = false;
        Ok(())
    }
    
    /// Handle incoming WebSocket messages.
    async fn handle_message(
        text: &str,
        config: &SubscriberConfig,
        event_sender: &broadcast::Sender<ProgramEvent>,
    ) -> SubscriberResult<()> {
        debug!("Received message: {}", text);
        
        let value: Value = serde_json::from_str(text)?;
        
        // Handle subscription confirmations
        if let Some(result) = value.get("result") {
            if result.is_number() {
                debug!("Subscription confirmed with ID: {}", result);
                return Ok(());
            }
        }
        
        // Handle notifications
        if let Some(method) = value.get("method") {
            if let Ok(ws_message) = serde_json::from_value::<WebSocketMessage>(value) {
                Self::process_notification(ws_message, config, event_sender).await?;
            }
        }
        
        Ok(())
    }
    
    /// Process WebSocket notifications and convert to program events.
    async fn process_notification(
        message: WebSocketMessage,
        config: &SubscriberConfig,
        event_sender: &broadcast::Sender<ProgramEvent>,
    ) -> SubscriberResult<()> {
        match message {
            WebSocketMessage::ProgramNotification { params } => {
                if let Ok(account_pubkey) = params.result.value.pubkey.parse::<Pubkey>() {
                    if let Ok(owner_pubkey) = params.result.value.account.owner.parse::<Pubkey>() {
                        // Find the program config
                        if let Some(program_config) = config.programs.iter().find(|p| p.id == owner_pubkey) {
                            let event = ProgramEvent::new(
                                owner_pubkey,
                                program_config.name.clone(),
                                EventType::AccountChange,
                                EventData::AccountChange {
                                    account: account_pubkey,
                                    balance_before: None,
                                    balance_after: Some(params.result.value.account.lamports),
                                    data_size_change: 0, // Would need more info to calculate
                                    owner: owner_pubkey,
                                },
                            ).with_slot(params.result.context.slot);
                            
                            if let Err(e) = event_sender.send(event) {
                                error!("Failed to send program event: {}", e);
                            }
                        }
                    }
                }
            }
            
            WebSocketMessage::LogsNotification { params } => {
                if let Ok(signature) = params.result.value.signature.parse() {
                    for log in &params.result.value.logs {
                        // Parse program ID from logs
                        if let Some(program_id) = Self::extract_program_id_from_log(log) {
                            if let Some(program_config) = config.programs.iter().find(|p| p.id == program_id) {
                                let event = ProgramEvent::new(
                                    program_id,
                                    program_config.name.clone(),
                                    EventType::LogEntry,
                                    EventData::LogEntry {
                                        message: log.clone(),
                                        level: None, // Could parse this from log content
                                        instruction_index: None,
                                    },
                                ).with_slot(params.result.context.slot)
                                 .with_signature(Some(signature));
                                
                                if let Err(e) = event_sender.send(event) {
                                    error!("Failed to send log event: {}", e);
                                }
                            }
                        }
                    }
                }
            }
            
            _ => {
                debug!("Unhandled notification type");
            }
        }
        
        Ok(())
    }
    
    /// Extract program ID from log message.
    fn extract_program_id_from_log(log: &str) -> Option<Pubkey> {
        // Simple pattern matching for program invocation logs
        if log.contains("Program ") && log.contains(" invoke") {
            let parts: Vec<&str> = log.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(pubkey) = parts[1].parse::<Pubkey>() {
                    return Some(pubkey);
                }
            }
        }
        None
    }
    
    /// Check if the client is connected.
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }
    
    /// Get the event receiver for listening to program events.
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<ProgramEvent> {
        self.event_sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProgramConfig, SubscriptionFilters};
    use url::Url;
    
    #[test]
    fn test_client_creation() {
        let config = SubscriberConfig {
            rpc_url: "https://api.mainnet-beta.solana.com".parse().unwrap(),
            ws_url: "wss://api.mainnet-beta.solana.com".parse().unwrap(),
            timeout_seconds: 30,
            max_reconnect_attempts: 5,
            reconnect_delay_seconds: 5,
            programs: vec![ProgramConfig {
                id: Pubkey::new_unique(),
                name: "Test Program".to_string(),
                monitor_accounts: true,
                monitor_transactions: true,
                monitor_logs: true,
                instruction_filters: None,
            }],
            filters: SubscriptionFilters::default(),
        };
        
        let client = SolanaWebSocketClient::new(config);
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_extract_program_id_from_log() {
        let log = "Program 11111111111111111111111111111111 invoke [1]";
        let program_id = SolanaWebSocketClient::extract_program_id_from_log(log);
        assert!(program_id.is_some());
    }
} 