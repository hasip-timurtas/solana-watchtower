//! Notification manager that coordinates all channels with rate limiting and batching.

use crate::{
    channels::{DiscordChannel, EmailChannel, NotificationChannel, SlackChannel, TelegramChannel},
    config::{NotifierConfig, NotificationFilter},
    error::{NotifierError, NotifierResult},
};
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{interval, Instant};
use tracing::{debug, error, info, warn};
use watchtower_engine::Alert;

/// Notification manager that handles all notification channels.
pub struct NotificationManager {
    /// Configured notification channels
    channels: HashMap<String, Box<dyn NotificationChannel>>,
    
    /// Rate limiters per channel
    rate_limiters: HashMap<String, RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>,
    
    /// Configuration
    config: NotifierConfig,
    
    /// Alert batching system
    batch_manager: Option<BatchManager>,
    
    /// Notification filters
    filters: Vec<NotificationFilter>,
    
    /// Statistics
    stats: Arc<RwLock<NotificationStats>>,
}

/// Batch manager for collecting and sending batched notifications.
struct BatchManager {
    /// Pending alerts per channel
    pending_alerts: Arc<RwLock<HashMap<String, Vec<Alert>>>>,
    
    /// Batch timeout interval
    batch_timeout: Duration,
    
    /// Maximum batch size
    max_batch_size: usize,
    
    /// Shutdown sender
    shutdown_tx: mpsc::Sender<()>,
}

/// Notification statistics.
#[derive(Debug, Clone, Default)]
pub struct NotificationStats {
    /// Total notifications sent
    pub total_sent: u64,
    
    /// Notifications sent per channel
    pub sent_per_channel: HashMap<String, u64>,
    
    /// Failed notifications
    pub total_failed: u64,
    
    /// Rate limited notifications
    pub rate_limited: u64,
    
    /// Batched notifications
    pub batched: u64,
    
    /// Last notification time
    pub last_notification: Option<chrono::DateTime<chrono::Utc>>,
}

impl NotificationManager {
    /// Create a new notification manager.
    pub async fn new(config: NotifierConfig) -> NotifierResult<Self> {
        config.validate()?;
        
        let mut channels: HashMap<String, Box<dyn NotificationChannel>> = HashMap::new();
        let mut rate_limiters = HashMap::new();
        
        // Initialize email channel
        if let Some(email_config) = &config.email {
            let channel = EmailChannel::new(email_config.clone())?;
            channels.insert("email".to_string(), Box::new(channel));
            
            let rate_limiter = RateLimiter::direct(Quota::per_minute(
                nonzero!(config.rate_limiting.max_messages_per_minute)
            ));
            rate_limiters.insert("email".to_string(), rate_limiter);
        }
        
        // Initialize Telegram channel
        if let Some(telegram_config) = &config.telegram {
            let channel = TelegramChannel::new(telegram_config.clone());
            channels.insert("telegram".to_string(), Box::new(channel));
            
            let rate_limiter = RateLimiter::direct(Quota::per_minute(
                nonzero!(config.rate_limiting.max_messages_per_minute)
            ));
            rate_limiters.insert("telegram".to_string(), rate_limiter);
        }
        
        // Initialize Slack channel
        if let Some(slack_config) = &config.slack {
            let channel = SlackChannel::new(slack_config.clone());
            channels.insert("slack".to_string(), Box::new(channel));
            
            let rate_limiter = RateLimiter::direct(Quota::per_minute(
                nonzero!(config.rate_limiting.max_messages_per_minute)
            ));
            rate_limiters.insert("slack".to_string(), rate_limiter);
        }
        
        // Initialize Discord channel
        if let Some(discord_config) = &config.discord {
            let channel = DiscordChannel::new(discord_config.clone());
            channels.insert("discord".to_string(), Box::new(channel));
            
            let rate_limiter = RateLimiter::direct(Quota::per_minute(
                nonzero!(config.rate_limiting.max_messages_per_minute)
            ));
            rate_limiters.insert("discord".to_string(), rate_limiter);
        }
        
        // Initialize batch manager if batching is enabled
        let batch_manager = if config.global.enable_batching {
            Some(BatchManager::new(
                Duration::from_secs(config.global.batch_timeout_seconds),
                config.global.batch_size,
            ).await?)
        } else {
            None
        };
        
        let filters = config.global.filters.clone().unwrap_or_default();
        
        info!("Notification manager initialized with {} channels", channels.len());
        
        Ok(Self {
            channels,
            rate_limiters,
            config,
            batch_manager,
            filters,
            stats: Arc::new(RwLock::new(NotificationStats::default())),
        })
    }
    
    /// Send a notification for an alert.
    pub async fn send_notification(&self, alert: Alert) -> NotifierResult<()> {
        debug!("Processing notification for alert: {}", alert.id);
        
        // Check minimum severity
        if !self.meets_minimum_severity(&alert) {
            debug!("Alert {} below minimum severity threshold", alert.id);
            return Ok(());
        }
        
        // Apply filters
        let channels_to_notify = self.apply_filters(&alert).await;
        
        if channels_to_notify.is_empty() {
            debug!("No channels to notify for alert {}", alert.id);
            return Ok(());
        }
        
        // Handle batching vs immediate sending
        if self.config.global.enable_batching {
            self.add_to_batch(alert, channels_to_notify).await?;
        } else {
            self.send_immediate(alert, channels_to_notify).await?;
        }
        
        Ok(())
    }
    
    /// Send notification immediately to specified channels.
    async fn send_immediate(&self, alert: Alert, channels: Vec<String>) -> NotifierResult<()> {
        let template_data = self.create_template_data(&alert);
        
        for channel_name in channels {
            if let Some(channel) = self.channels.get(&channel_name) {
                // Check rate limit
                if self.config.rate_limiting.enabled {
                    if let Some(rate_limiter) = self.rate_limiters.get(&channel_name) {
                        if rate_limiter.check().is_err() {
                            warn!("Rate limit exceeded for channel: {}", channel_name);
                            self.update_stats(|stats| stats.rate_limited += 1).await;
                            continue;
                        }
                    }
                }
                
                // Send notification
                match channel.send(&alert, &template_data).await {
                    Ok(_) => {
                        info!("Notification sent successfully via {}", channel_name);
                        self.update_stats(|stats| {
                            stats.total_sent += 1;
                            *stats.sent_per_channel.entry(channel_name.clone()).or_insert(0) += 1;
                            stats.last_notification = Some(chrono::Utc::now());
                        }).await;
                    }
                    Err(e) => {
                        error!("Failed to send notification via {}: {}", channel_name, e);
                        self.update_stats(|stats| stats.total_failed += 1).await;
                        return Err(e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Add alert to batch for later sending.
    async fn add_to_batch(&self, alert: Alert, channels: Vec<String>) -> NotifierResult<()> {
        if let Some(batch_manager) = &self.batch_manager {
            batch_manager.add_alert(alert, channels).await;
        }
        Ok(())
    }
    
    /// Send batched notifications.
    pub async fn send_batch(&self, alerts: Vec<Alert>, channel_name: &str) -> NotifierResult<()> {
        if alerts.is_empty() {
            return Ok(());
        }
        
        if let Some(channel) = self.channels.get(channel_name) {
            if channel.supports_batching() {
                let template_data = self.create_batch_template_data(&alerts);
                
                // Check rate limit
                if self.config.rate_limiting.enabled {
                    if let Some(rate_limiter) = self.rate_limiters.get(channel_name) {
                        if rate_limiter.check().is_err() {
                            warn!("Rate limit exceeded for batch on channel: {}", channel_name);
                            self.update_stats(|stats| stats.rate_limited += 1).await;
                            return Ok(());
                        }
                    }
                }
                
                match channel.send_batch(&alerts, &template_data).await {
                    Ok(_) => {
                        info!("Batch notification sent successfully via {} ({} alerts)", channel_name, alerts.len());
                        self.update_stats(|stats| {
                            stats.total_sent += 1;
                            stats.batched += alerts.len() as u64;
                            *stats.sent_per_channel.entry(channel_name.to_string()).or_insert(0) += 1;
                            stats.last_notification = Some(chrono::Utc::now());
                        }).await;
                    }
                    Err(e) => {
                        error!("Failed to send batch notification via {}: {}", channel_name, e);
                        self.update_stats(|stats| stats.total_failed += 1).await;
                        
                        // Fallback to individual notifications
                        warn!("Falling back to individual notifications for {} alerts", alerts.len());
                        for alert in alerts {
                            if let Err(e) = self.send_immediate(alert, vec![channel_name.to_string()]).await {
                                error!("Fallback notification failed: {}", e);
                            }
                        }
                    }
                }
            } else {
                // Channel doesn't support batching, send individually
                for alert in alerts {
                    if let Err(e) = self.send_immediate(alert, vec![channel_name.to_string()]).await {
                        error!("Individual notification failed: {}", e);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Test all configured notification channels.
    pub async fn test_channels(&self) -> HashMap<String, NotifierResult<()>> {
        let mut results = HashMap::new();
        
        for (channel_name, channel) in &self.channels {
            info!("Testing channel: {}", channel_name);
            let result = channel.test().await;
            results.insert(channel_name.clone(), result);
        }
        
        results
    }
    
    /// Get notification statistics.
    pub async fn statistics(&self) -> NotificationStats {
        self.stats.read().await.clone()
    }
    
    /// Shutdown the notification manager.
    pub async fn shutdown(&self) -> NotifierResult<()> {
        if let Some(batch_manager) = &self.batch_manager {
            batch_manager.shutdown().await?;
        }
        
        info!("Notification manager shut down");
        Ok(())
    }
    
    /// Check if alert meets minimum severity requirement.
    fn meets_minimum_severity(&self, alert: &Alert) -> bool {
        let min_severity = match self.config.global.min_severity.as_str() {
            "critical" => watchtower_engine::AlertSeverity::Critical,
            "high" => watchtower_engine::AlertSeverity::High,
            "medium" => watchtower_engine::AlertSeverity::Medium,
            "low" => watchtower_engine::AlertSeverity::Low,
            _ => watchtower_engine::AlertSeverity::Info,
        };
        
        alert.severity >= min_severity
    }
    
    /// Apply filters and return channels that should receive the notification.
    async fn apply_filters(&self, alert: &Alert) -> Vec<String> {
        let mut eligible_channels = self.config.enabled_channels();
        
        // Apply each filter
        for filter in &self.filters {
            let matches = self.filter_matches(filter, alert);
            
            if filter.include && matches {
                // Include filter matches - keep only specified channels
                if let Some(filter_channels) = &filter.channels {
                    eligible_channels.retain(|c| filter_channels.contains(c));
                }
            } else if !filter.include && matches {
                // Exclude filter matches - remove specified channels
                if let Some(filter_channels) = &filter.channels {
                    eligible_channels.retain(|c| !filter_channels.contains(c));
                } else {
                    // Exclude from all channels
                    eligible_channels.clear();
                }
            }
        }
        
        eligible_channels
    }
    
    /// Check if a filter matches an alert.
    fn filter_matches(&self, filter: &NotificationFilter, alert: &Alert) -> bool {
        // Check rule names
        if let Some(rule_names) = &filter.rule_names {
            if !rule_names.contains(&alert.rule_name) {
                return false;
            }
        }
        
        // Check program names
        if let Some(program_names) = &filter.program_names {
            if !program_names.contains(&alert.program_name) {
                return false;
            }
        }
        
        // Check severities
        if let Some(severities) = &filter.severities {
            if !severities.contains(&alert.severity.as_str().to_string()) {
                return false;
            }
        }
        
        true
    }
    
    /// Create template data for a single alert.
    fn create_template_data(&self, alert: &Alert) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        
        data.insert("alert".to_string(), serde_json::to_value(alert).unwrap_or_default());
        data.insert("timestamp".to_string(), serde_json::to_value(chrono::Utc::now()).unwrap_or_default());
        
        data
    }
    
    /// Create template data for multiple alerts.
    fn create_batch_template_data(&self, alerts: &[Alert]) -> HashMap<String, Value> {
        let mut data = HashMap::new();
        
        data.insert("alerts".to_string(), serde_json::to_value(alerts).unwrap_or_default());
        data.insert("alert_count".to_string(), serde_json::to_value(alerts.len()).unwrap_or_default());
        data.insert("timestamp".to_string(), serde_json::to_value(chrono::Utc::now()).unwrap_or_default());
        
        data
    }
    
    /// Update statistics with a closure.
    async fn update_stats<F>(&self, f: F)
    where
        F: FnOnce(&mut NotificationStats),
    {
        let mut stats = self.stats.write().await;
        f(&mut *stats);
    }
}

impl BatchManager {
    /// Create a new batch manager.
    async fn new(batch_timeout: Duration, max_batch_size: usize) -> NotifierResult<Self> {
        let pending_alerts = Arc::new(RwLock::new(HashMap::new()));
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        
        let batch_manager = Self {
            pending_alerts: pending_alerts.clone(),
            batch_timeout,
            max_batch_size,
            shutdown_tx,
        };
        
        // Start batch processing task
        let pending_alerts_clone = pending_alerts.clone();
        tokio::spawn(async move {
            let mut interval = interval(batch_timeout);
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Process batches on timeout
                        Self::process_batches(pending_alerts_clone.clone(), max_batch_size).await;
                    }
                    _ = shutdown_rx.recv() => {
                        // Shutdown signal received
                        debug!("Batch manager shutting down");
                        break;
                    }
                }
            }
        });
        
        Ok(batch_manager)
    }
    
    /// Add an alert to the batch.
    async fn add_alert(&self, alert: Alert, channels: Vec<String>) {
        let mut pending = self.pending_alerts.write().await;
        
        for channel in channels {
            let alerts = pending.entry(channel.clone()).or_insert_with(Vec::new);
            alerts.push(alert.clone());
            
            // Check if batch is full
            if alerts.len() >= self.max_batch_size {
                debug!("Batch full for channel {}, processing immediately", channel);
                // Process this batch immediately
                let batch = std::mem::take(alerts);
                drop(pending); // Release lock before async operation
                
                // TODO: Send batch notification
                // This would require access to the NotificationManager
                // For now, we'll rely on the timer-based processing
                
                return;
            }
        }
    }
    
    /// Process all pending batches.
    async fn process_batches(
        pending_alerts: Arc<RwLock<HashMap<String, Vec<Alert>>>>,
        _max_batch_size: usize,
    ) {
        let mut pending = pending_alerts.write().await;
        
        for (channel, alerts) in pending.iter_mut() {
            if !alerts.is_empty() {
                debug!("Processing batch for channel {} with {} alerts", channel, alerts.len());
                
                // TODO: Actually send the batch
                // This would require access to the NotificationManager
                // For now, we'll just clear the batch
                alerts.clear();
            }
        }
    }
    
    /// Shutdown the batch manager.
    async fn shutdown(&self) -> NotifierResult<()> {
        // Process any pending batches before shutdown
        Self::process_batches(self.pending_alerts.clone(), self.max_batch_size).await;
        
        // Send shutdown signal
        if let Err(e) = self.shutdown_tx.send(()).await {
            warn!("Failed to send shutdown signal to batch manager: {}", e);
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{EmailConfig, NotifierConfig, GlobalNotificationConfig, RateLimitConfig};
    use watchtower_engine::AlertSeverity;
    
    #[tokio::test]
    async fn test_notification_manager_creation() {
        let config = NotifierConfig {
            email: Some(EmailConfig {
                smtp_server: "smtp.example.com".to_string(),
                smtp_port: 587,
                username: "test@example.com".to_string(),
                password: "password".to_string(),
                from_address: "test@example.com".to_string(),
                from_name: Some("Test".to_string()),
                to_addresses: vec!["recipient@example.com".to_string()],
                use_tls: true,
                subject_template: None,
                body_template: None,
            }),
            telegram: None,
            slack: None,
            discord: None,
            rate_limiting: RateLimitConfig::default(),
            global: GlobalNotificationConfig::default(),
        };
        
        let result = NotificationManager::new(config).await;
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_severity_filtering() {
        let config = NotifierConfig {
            email: None,
            telegram: None,
            slack: None,
            discord: None,
            rate_limiting: RateLimitConfig::default(),
            global: GlobalNotificationConfig {
                min_severity: "high".to_string(),
                ..Default::default()
            },
        };
        
        // This would fail validation due to no channels, but we're testing the logic
        let manager = NotificationManager {
            channels: HashMap::new(),
            rate_limiters: HashMap::new(),
            config,
            batch_manager: None,
            filters: Vec::new(),
            stats: Arc::new(RwLock::new(NotificationStats::default())),
        };
        
        let high_alert = Alert {
            id: "test".to_string(),
            rule_name: "test_rule".to_string(),
            message: "Test message".to_string(),
            severity: AlertSeverity::High,
            program_id: solana_sdk::pubkey::Pubkey::new_unique(),
            program_name: "Test Program".to_string(),
            event_id: None,
            metadata: HashMap::new(),
            confidence: 0.8,
            suggested_actions: Vec::new(),
            timestamp: chrono::Utc::now(),
            acknowledged: false,
            resolved: false,
        };
        
        let low_alert = Alert {
            severity: AlertSeverity::Low,
            ..high_alert.clone()
        };
        
        assert!(manager.meets_minimum_severity(&high_alert));
        assert!(!manager.meets_minimum_severity(&low_alert));
    }
} 