//! Core monitoring engine that coordinates event processing, rule evaluation, and alerting.

use crate::{
    alerts::{Alert, AlertManager},
    metrics::{MetricsCollector, MetricsSnapshot},
    rules::{Rule, RuleContext, RuleResult},
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{debug, error, info, warn};
use watchtower_subscriber::ProgramEvent;

/// Core monitoring engine that processes events and evaluates rules.
pub struct MonitoringEngine {
    /// Registered rules
    rules: Arc<RwLock<Vec<Box<dyn Rule>>>>,
    
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    
    /// Alert manager
    alert_manager: Arc<AlertManager>,
    
    /// Event history for rule context
    event_history: Arc<DashMap<String, Vec<ProgramEvent>>>,
    
    /// Engine configuration
    config: EngineConfig,
    
    /// Event sender for alerts
    alert_sender: broadcast::Sender<Alert>,
    
    /// Engine state
    state: Arc<RwLock<EngineState>>,
}

/// Configuration for the monitoring engine.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Maximum events to keep in history per program
    pub max_history_events: usize,
    
    /// Maximum age of events to keep in history
    pub max_history_age: Duration,
    
    /// Interval for metrics snapshots
    pub metrics_interval: Duration,
    
    /// Maximum concurrent rule evaluations
    pub max_concurrent_evaluations: usize,
    
    /// Rule evaluation timeout
    pub rule_timeout: Duration,
    
    /// Whether to enable detailed logging
    pub debug_logging: bool,
}

/// Current state of the monitoring engine.
#[derive(Debug, Clone)]
pub struct EngineState {
    /// Whether the engine is running
    pub running: bool,
    
    /// Start time
    pub start_time: DateTime<Utc>,
    
    /// Total events processed
    pub events_processed: u64,
    
    /// Total rules evaluated
    pub rules_evaluated: u64,
    
    /// Total alerts generated
    pub alerts_generated: u64,
    
    /// Last metrics snapshot time
    pub last_metrics_snapshot: Option<DateTime<Utc>>,
    
    /// Performance statistics
    pub performance: PerformanceStats,
}

/// Performance statistics for the engine.
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    /// Average event processing time
    pub avg_event_processing_time: Duration,
    
    /// Average rule evaluation time
    pub avg_rule_evaluation_time: Duration,
    
    /// Peak events per second
    pub peak_events_per_second: f64,
    
    /// Current events per second
    pub current_events_per_second: f64,
    
    /// Memory usage (if available)
    pub memory_usage_bytes: Option<u64>,
}

/// Result of event processing.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// Number of rules evaluated
    pub rules_evaluated: usize,
    
    /// Number of alerts generated
    pub alerts_generated: usize,
    
    /// Processing duration
    pub duration: Duration,
    
    /// Any errors encountered
    pub errors: Vec<String>,
}

/// Errors that can occur in the monitoring engine.
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Engine is not running")]
    NotRunning,
    
    #[error("Rule evaluation timeout: {rule}")]
    RuleTimeout { rule: String },
    
    #[error("Failed to process event: {0}")]
    EventProcessing(String),
    
    #[error("Alert generation failed: {0}")]
    AlertGeneration(String),
    
    #[error("Metrics error: {0}")]
    Metrics(#[from] crate::metrics::MetricsError),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type EngineResult<T> = Result<T, EngineError>;

impl MonitoringEngine {
    /// Create a new monitoring engine.
    pub fn new(
        metrics: Arc<MetricsCollector>,
        alert_manager: Arc<AlertManager>,
        config: EngineConfig,
    ) -> Self {
        let (alert_sender, _) = broadcast::channel(1000);
        
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            metrics,
            alert_manager,
            event_history: Arc::new(DashMap::new()),
            config,
            alert_sender,
            state: Arc::new(RwLock::new(EngineState {
                running: false,
                start_time: Utc::now(),
                events_processed: 0,
                rules_evaluated: 0,
                alerts_generated: 0,
                last_metrics_snapshot: None,
                performance: PerformanceStats::default(),
            })),
        }
    }
    
    /// Add a rule to the engine.
    pub async fn add_rule(&self, rule: Box<dyn Rule>) {
        let mut rules = self.rules.write().await;
        info!("Adding rule: {}", rule.name());
        rules.push(rule);
    }
    
    /// Remove a rule from the engine.
    pub async fn remove_rule(&self, rule_name: &str) -> bool {
        let mut rules = self.rules.write().await;
        let initial_len = rules.len();
        rules.retain(|rule| rule.name() != rule_name);
        let removed = rules.len() != initial_len;
        
        if removed {
            info!("Removed rule: {}", rule_name);
        }
        
        removed
    }
    
    /// Get all registered rules.
    pub async fn list_rules(&self) -> Vec<String> {
        let rules = self.rules.read().await;
        rules.iter().map(|rule| rule.name().to_string()).collect()
    }
    
    /// Start the monitoring engine.
    pub async fn start(&self) -> EngineResult<()> {
        let mut state = self.state.write().await;
        if state.running {
            return Ok(());
        }
        
        state.running = true;
        state.start_time = Utc::now();
        info!("Monitoring engine started");
        
        Ok(())
    }
    
    /// Stop the monitoring engine.
    pub async fn stop(&self) -> EngineResult<()> {
        let mut state = self.state.write().await;
        if !state.running {
            return Ok(());
        }
        
        state.running = false;
        info!("Monitoring engine stopped");
        
        Ok(())
    }
    
    /// Process a program event through all registered rules.
    pub async fn process_event(&self, event: ProgramEvent) -> EngineResult<ProcessingResult> {
        let start_time = Instant::now();
        let mut result = ProcessingResult {
            rules_evaluated: 0,
            alerts_generated: 0,
            duration: Duration::default(),
            errors: Vec::new(),
        };
        
        // Check if engine is running
        {
            let state = self.state.read().await;
            if !state.running {
                return Err(EngineError::NotRunning);
            }
        }
        
        // Record event metrics
        self.metrics.record_event(&event.program_name, event.event_type.as_str());
        
        // Add event to history
        self.add_to_history(event.clone()).await;
        
        // Create rule context
        let context = self.create_rule_context(&event).await;
        
        // Evaluate rules
        let rules = self.rules.read().await;
        let enabled_rules: Vec<_> = rules.iter().filter(|rule| rule.is_enabled()).collect();
        drop(rules);
        
        if self.config.debug_logging {
            debug!("Evaluating {} rules for event {}", enabled_rules.len(), event.id);
        }
        
        // Process rules concurrently with timeout
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent_evaluations));
        let mut rule_tasks = Vec::new();
        
        for rule in enabled_rules {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let rule_name = rule.name().to_string();
            let event_clone = event.clone();
            let context_clone = context.clone();
            let metrics_clone = self.metrics.clone();
            let rule_timeout = self.config.rule_timeout;
            
            let task = tokio::spawn(async move {
                let _permit = permit; // Keep permit alive
                let rule_start = Instant::now();
                
                let evaluation_result = tokio::time::timeout(
                    rule_timeout,
                    rule.evaluate(&event_clone, &context_clone),
                ).await;
                
                let duration = rule_start.elapsed();
                
                match evaluation_result {
                    Ok(rule_result) => {
                        metrics_clone.record_rule_evaluation(&rule_name, duration, rule_result.triggered);
                        Ok((rule_name, rule_result))
                    }
                    Err(_) => {
                        error!("Rule evaluation timeout: {}", rule_name);
                        Err(EngineError::RuleTimeout { rule: rule_name })
                    }
                }
            });
            
            rule_tasks.push(task);
        }
        
        // Wait for all rule evaluations to complete
        for task in rule_tasks {
            match task.await {
                Ok(Ok((rule_name, rule_result))) => {
                    result.rules_evaluated += 1;
                    
                    if rule_result.triggered {
                        // Generate alert
                        match self.generate_alert(rule_result, &event).await {
                            Ok(_) => {
                                result.alerts_generated += 1;
                                self.metrics.record_alert(&rule_name, rule_result.severity.as_str());
                            }
                            Err(e) => {
                                result.errors.push(format!("Alert generation failed for rule {}: {}", rule_name, e));
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    result.errors.push(e.to_string());
                }
                Err(e) => {
                    result.errors.push(format!("Rule task failed: {}", e));
                }
            }
        }
        
        // Update state
        {
            let mut state = self.state.write().await;
            state.events_processed += 1;
            state.rules_evaluated += result.rules_evaluated as u64;
            state.alerts_generated += result.alerts_generated as u64;
        }
        
        result.duration = start_time.elapsed();
        
        // Record processing latency
        self.metrics.record_event_processing_time(result.duration.as_secs_f64());
        
        if self.config.debug_logging {
            debug!(
                "Processed event {} in {:?}: {} rules evaluated, {} alerts generated",
                event.id, result.duration, result.rules_evaluated, result.alerts_generated
            );
        }
        
        Ok(result)
    }
    
    /// Add event to history for rule context.
    async fn add_to_history(&self, event: ProgramEvent) {
        let program_key = format!("{}_{}", event.program_id, event.program_name);
        
        let mut entry = self.event_history.entry(program_key).or_insert_with(Vec::new);
        entry.push(event);
        
        // Trim history to configured limits
        let cutoff_time = Utc::now() - chrono::Duration::from_std(self.config.max_history_age).unwrap();
        entry.retain(|e| e.timestamp >= cutoff_time);
        
        if entry.len() > self.config.max_history_events {
            let excess = entry.len() - self.config.max_history_events;
            entry.drain(0..excess);
        }
    }
    
    /// Create rule context for evaluation.
    async fn create_rule_context(&self, event: &ProgramEvent) -> RuleContext {
        let program_key = format!("{}_{}", event.program_id, event.program_name);
        
        let recent_events = self.event_history
            .get(&program_key)
            .map(|entry| entry.clone())
            .unwrap_or_default();
        
        let metrics_snapshot = self.metrics.snapshot();
        
        RuleContext {
            recent_events,
            metrics: metrics_snapshot.values,
            config: HashMap::new(), // Could be populated from configuration
            timestamp: Utc::now(),
        }
    }
    
    /// Generate an alert from a rule result.
    async fn generate_alert(&self, rule_result: RuleResult, event: &ProgramEvent) -> EngineResult<()> {
        let alert = Alert {
            id: uuid::Uuid::new_v4().to_string(),
            rule_name: rule_result.rule_name,
            message: rule_result.message.unwrap_or_else(|| "Rule triggered".to_string()),
            severity: rule_result.severity,
            program_id: event.program_id,
            program_name: event.program_name.clone(),
            event_id: Some(event.id.clone()),
            metadata: rule_result.metadata,
            confidence: rule_result.confidence,
            suggested_actions: rule_result.suggested_actions,
            timestamp: rule_result.timestamp,
            acknowledged: false,
            resolved: false,
        };
        
        // Send alert through manager
        self.alert_manager.send_alert(alert.clone()).await
            .map_err(|e| EngineError::AlertGeneration(e.to_string()))?;
        
        // Broadcast alert to subscribers
        if let Err(e) = self.alert_sender.send(alert) {
            warn!("Failed to broadcast alert: {}", e);
        }
        
        Ok(())
    }
    
    /// Get current engine state.
    pub async fn state(&self) -> EngineState {
        self.state.read().await.clone()
    }
    
    /// Get metrics snapshot.
    pub fn metrics_snapshot(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }
    
    /// Subscribe to alerts.
    pub fn subscribe_to_alerts(&self) -> broadcast::Receiver<Alert> {
        self.alert_sender.subscribe()
    }
    
    /// Get event history for a program.
    pub async fn get_event_history(&self, program_id: &str, program_name: &str) -> Vec<ProgramEvent> {
        let program_key = format!("{}_{}", program_id, program_name);
        self.event_history
            .get(&program_key)
            .map(|entry| entry.clone())
            .unwrap_or_default()
    }
    
    /// Clear event history.
    pub async fn clear_history(&self) {
        self.event_history.clear();
        info!("Cleared event history");
    }
    
    /// Get engine statistics.
    pub async fn statistics(&self) -> EngineStatistics {
        let state = self.state.read().await;
        let uptime = Utc::now() - state.start_time;
        
        EngineStatistics {
            uptime: uptime.to_std().unwrap_or_default(),
            events_processed: state.events_processed,
            rules_evaluated: state.rules_evaluated,
            alerts_generated: state.alerts_generated,
            rules_registered: self.rules.read().await.len(),
            programs_monitored: self.event_history.len(),
            performance: state.performance.clone(),
        }
    }
}

/// Engine statistics for monitoring and debugging.
#[derive(Debug, Clone)]
pub struct EngineStatistics {
    /// Engine uptime
    pub uptime: Duration,
    
    /// Total events processed
    pub events_processed: u64,
    
    /// Total rules evaluated
    pub rules_evaluated: u64,
    
    /// Total alerts generated
    pub alerts_generated: u64,
    
    /// Number of registered rules
    pub rules_registered: usize,
    
    /// Number of programs being monitored
    pub programs_monitored: usize,
    
    /// Performance statistics
    pub performance: PerformanceStats,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_history_events: 1000,
            max_history_age: Duration::from_secs(3600), // 1 hour
            metrics_interval: Duration::from_secs(60),   // 1 minute
            max_concurrent_evaluations: 100,
            rule_timeout: Duration::from_secs(30),
            debug_logging: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        alerts::AlertManager,
        metrics::MetricsCollector,
        rules::{AlertSeverity, LargeTransactionRule},
    };
    use watchtower_subscriber::{EventData, EventType, ProgramEvent};
    use solana_sdk::pubkey::Pubkey;
    
    #[tokio::test]
    async fn test_engine_creation() {
        let metrics = Arc::new(MetricsCollector::new().unwrap());
        let alert_manager = Arc::new(AlertManager::new());
        let config = EngineConfig::default();
        
        let engine = MonitoringEngine::new(metrics, alert_manager, config);
        assert!(!engine.state().await.running);
    }
    
    #[tokio::test]
    async fn test_rule_management() {
        let metrics = Arc::new(MetricsCollector::new().unwrap());
        let alert_manager = Arc::new(AlertManager::new());
        let config = EngineConfig::default();
        
        let engine = MonitoringEngine::new(metrics, alert_manager, config);
        
        // Add rule
        let rule = Box::new(LargeTransactionRule::new(1.0, 1000000));
        engine.add_rule(rule).await;
        
        let rules = engine.list_rules().await;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0], "large_transaction");
        
        // Remove rule
        let removed = engine.remove_rule("large_transaction").await;
        assert!(removed);
        
        let rules = engine.list_rules().await;
        assert_eq!(rules.len(), 0);
    }
    
    #[tokio::test]
    async fn test_event_processing() {
        let metrics = Arc::new(MetricsCollector::new().unwrap());
        let alert_manager = Arc::new(AlertManager::new());
        let config = EngineConfig::default();
        
        let engine = MonitoringEngine::new(metrics, alert_manager, config);
        engine.start().await.unwrap();
        
        let event = ProgramEvent::new(
            Pubkey::new_unique(),
            "Test Program".to_string(),
            EventType::TokenTransfer,
            EventData::TokenTransfer {
                from: Pubkey::new_unique(),
                to: Pubkey::new_unique(),
                amount: 1000,
                mint: Pubkey::new_unique(),
                decimals: 6,
            },
        );
        
        let result = engine.process_event(event).await;
        assert!(result.is_ok());
        
        let stats = engine.statistics().await;
        assert_eq!(stats.events_processed, 1);
    }
} 