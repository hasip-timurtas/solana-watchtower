use crate::AppState;
use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;
use watchtower_engine::{Alert, AlertManager};

/// WebSocket connection info
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub id: String,
    pub sender: tokio::sync::mpsc::UnboundedSender<WebSocketMessage>,
    pub last_ping: std::time::Instant,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    Ping,
    Pong,
    Alert { data: AlertNotification },
    Status { data: StatusUpdate },
    Metrics { data: MetricsUpdate },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertNotification {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub program_id: String,
    pub timestamp: String,
    pub rule_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub engine_status: String,
    pub alert_count: usize,
    pub active_rules: usize,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsUpdate {
    pub timestamp: i64,
    pub metrics: HashMap<String, f64>,
}

/// Handle new WebSocket connection
pub async fn handle_websocket(socket: WebSocket, state: AppState) {
    let connection_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection: {}", connection_id);

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<WebSocketMessage>();

    // Store connection
    let connection = WebSocketConnection {
        id: connection_id.clone(),
        sender: tx,
        last_ping: std::time::Instant::now(),
    };

    state.ws_connections.write().await.insert(connection_id.clone(), connection);

    // Task to send messages from the channel to WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let serialized = match serde_json::to_string(&msg) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to serialize WebSocket message: {}", e);
                    continue;
                }
            };

            if sender.send(Message::Text(serialized)).await.is_err() {
                warn!("Failed to send WebSocket message, connection likely closed");
                break;
            }
        }
    });

    // Task to handle incoming messages
    let connection_id_clone = connection_id.clone();
    let ws_connections = state.ws_connections.clone();
    let receive_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = handle_websocket_message(&text, &connection_id_clone, &ws_connections).await {
                        error!("Error handling WebSocket message: {}", e);
                    }
                }
                Ok(Message::Ping(ping)) => {
                    info!("Received ping from {}", connection_id_clone);
                    // Axum handles pong automatically
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection {} closed", connection_id_clone);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error for connection {}: {}", connection_id_clone, e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = receive_task => {},
    }

    // Clean up connection
    state.ws_connections.write().await.remove(&connection_id);
    info!("WebSocket connection {} cleaned up", connection_id);
}

/// Handle incoming WebSocket message
async fn handle_websocket_message(
    text: &str,
    connection_id: &str,
    ws_connections: &Arc<RwLock<HashMap<String, WebSocketConnection>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let message: WebSocketMessage = serde_json::from_str(text)?;

    match message {
        WebSocketMessage::Ping => {
            // Update last ping time and send pong
            if let Some(connection) = ws_connections.write().await.get_mut(connection_id) {
                connection.last_ping = std::time::Instant::now();
                let _ = connection.sender.send(WebSocketMessage::Pong);
            }
        }
        WebSocketMessage::Pong => {
            // Update last ping time
            if let Some(connection) = ws_connections.write().await.get_mut(connection_id) {
                connection.last_ping = std::time::Instant::now();
            }
        }
        _ => {
            warn!("Unexpected message type from client: {:?}", message);
        }
    }

    Ok(())
}

/// Broadcast message to all connected WebSocket clients
pub async fn broadcast_to_websockets(
    message: WebSocketMessage,
    ws_connections: &Arc<RwLock<HashMap<String, WebSocketConnection>>>,
) {
    let connections = ws_connections.read().await;
    let mut failed_connections = Vec::new();

    for (connection_id, connection) in connections.iter() {
        if connection.sender.send(message.clone()).is_err() {
            failed_connections.push(connection_id.clone());
        }
    }

    // Clean up failed connections
    drop(connections);
    if !failed_connections.is_empty() {
        let mut connections = ws_connections.write().await;
        for connection_id in failed_connections {
            connections.remove(&connection_id);
            info!("Removed failed WebSocket connection: {}", connection_id);
        }
    }
}

/// Background task to send periodic heartbeats
pub async fn websocket_heartbeat_task(
    ws_connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        let ping_message = WebSocketMessage::Ping;
        broadcast_to_websockets(ping_message, &ws_connections).await;

        // Remove stale connections (no pong received in last 60 seconds)
        let now = std::time::Instant::now();
        let mut stale_connections = Vec::new();
        
        {
            let connections = ws_connections.read().await;
            for (connection_id, connection) in connections.iter() {
                if now.duration_since(connection.last_ping) > Duration::from_secs(60) {
                    stale_connections.push(connection_id.clone());
                }
            }
        }

        if !stale_connections.is_empty() {
            let mut connections = ws_connections.write().await;
            for connection_id in stale_connections {
                connections.remove(&connection_id);
                info!("Removed stale WebSocket connection: {}", connection_id);
            }
        }
    }
}

/// Background task to broadcast alerts to WebSocket clients
pub async fn alert_broadcast_task(
    alert_manager: Arc<AlertManager>,
    ws_connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
) {
    let mut alert_receiver = alert_manager.subscribe().await;
    
    while let Ok(alert) = alert_receiver.recv().await {
        let notification = AlertNotification {
            id: alert.id.clone(),
            severity: format!("{:?}", alert.severity),
            message: alert.message.clone(),
            program_id: alert.program_id.to_string(),
            timestamp: alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            rule_name: alert.rule_name.clone(),
        };

        let message = WebSocketMessage::Alert { data: notification };
        broadcast_to_websockets(message, &ws_connections).await;
    }
}

/// Send status updates to WebSocket clients
pub async fn send_status_update(
    status: StatusUpdate,
    ws_connections: &Arc<RwLock<HashMap<String, WebSocketConnection>>>,
) {
    let message = WebSocketMessage::Status { data: status };
    broadcast_to_websockets(message, ws_connections).await;
}

/// Send metrics updates to WebSocket clients
pub async fn send_metrics_update(
    metrics: MetricsUpdate,
    ws_connections: &Arc<RwLock<HashMap<String, WebSocketConnection>>>,
) {
    let message = WebSocketMessage::Metrics { data: metrics };
    broadcast_to_websockets(message, ws_connections).await;
} 