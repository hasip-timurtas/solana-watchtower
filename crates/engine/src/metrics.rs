//! Metrics collection and aggregation for Solana program monitoring.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use prometheus::{
    GaugeVec, Histogram, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec, Registry,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Metrics collector for program monitoring.
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// Prometheus registry
    registry: Arc<Registry>,

    /// Custom metrics storage
    custom_metrics: Arc<DashMap<String, MetricValue>>,

    /// Built-in counters
    counters: MetricsCounters,

    /// Built-in gauges
    gauges: MetricsGauges,

    /// Built-in histograms
    histograms: MetricsHistograms,

    /// Sliding window metrics
    windows: Arc<DashMap<String, SlidingWindow>>,
}

/// Built-in counter metrics.
#[derive(Debug, Clone)]
pub struct MetricsCounters {
    /// Total events processed
    pub events_total: IntCounterVec,

    /// Total alerts generated
    pub alerts_total: IntCounterVec,

    /// Transaction count by program
    pub transactions_total: IntCounterVec,

    /// Failed transactions
    pub failed_transactions_total: IntCounterVec,

    /// Rule evaluations
    pub rule_evaluations_total: IntCounterVec,
}

/// Built-in gauge metrics.
#[derive(Debug, Clone)]
pub struct MetricsGauges {
    /// Active connections
    pub active_connections: IntGauge,

    /// Total value locked by program
    pub total_value_locked: GaugeVec,

    /// Current token prices
    pub token_prices: GaugeVec,

    /// Program account count
    pub program_accounts: IntGaugeVec,

    /// Recent failure rate
    pub failure_rate: GaugeVec,
}

/// Built-in histogram metrics.
#[derive(Debug, Clone)]
pub struct MetricsHistograms {
    /// Transaction amounts
    pub transaction_amounts: HistogramVec,

    /// Rule evaluation duration
    pub rule_evaluation_duration: HistogramVec,

    /// Event processing latency
    pub event_processing_latency: Histogram,
}

/// Custom metric value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricValue {
    Counter(f64),
    Gauge(f64),
    Histogram {
        buckets: Vec<(f64, u64)>,
        sum: f64,
        count: u64,
    },
}

/// Sliding window for time-based metrics.
#[derive(Debug)]
pub struct SlidingWindow {
    /// Window duration
    duration: Duration,

    /// Data points with timestamps
    data: Vec<(Instant, f64)>,

    /// Maximum number of data points to keep
    max_points: usize,
}

/// Metrics snapshot for rule evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Timestamp of snapshot
    pub timestamp: DateTime<Utc>,

    /// Current metric values
    pub values: HashMap<String, f64>,

    /// Window-based aggregations
    pub windows: HashMap<String, WindowStats>,
}

/// Statistical aggregations for sliding windows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowStats {
    /// Average value
    pub avg: f64,

    /// Minimum value
    pub min: f64,

    /// Maximum value
    pub max: f64,

    /// Total sum
    pub sum: f64,

    /// Number of data points
    pub count: usize,

    /// Standard deviation
    pub std_dev: f64,

    /// Percentile values (50th, 90th, 95th, 99th)
    pub percentiles: HashMap<String, f64>,
}

/// Errors that can occur in metrics operations.
#[derive(Error, Debug)]
pub enum MetricsError {
    #[error("Metric not found: {0}")]
    NotFound(String),

    #[error("Invalid metric value: {0}")]
    InvalidValue(String),

    #[error("Prometheus error: {0}")]
    Prometheus(#[from] prometheus::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type MetricsResult<T> = Result<T, MetricsError>;

impl MetricsCollector {
    /// Create a new metrics collector.
    pub fn new() -> MetricsResult<Self> {
        let registry = Arc::new(Registry::new());

        let counters = MetricsCounters::new(&registry)?;
        let gauges = MetricsGauges::new(&registry)?;
        let histograms = MetricsHistograms::new(&registry)?;

        Ok(Self {
            registry,
            custom_metrics: Arc::new(DashMap::new()),
            counters,
            gauges,
            histograms,
            windows: Arc::new(DashMap::new()),
        })
    }

    /// Record an event being processed.
    pub fn record_event(&self, program_name: &str, event_type: &str) {
        self.counters
            .events_total
            .with_label_values(&[program_name, event_type])
            .inc();
    }

    /// Record an alert being generated.
    pub fn record_alert(&self, rule_name: &str, severity: &str) {
        self.counters
            .alerts_total
            .with_label_values(&[rule_name, severity])
            .inc();
    }

    /// Record a transaction.
    pub fn record_transaction(&self, program_name: &str, success: bool, amount: f64) {
        self.counters
            .transactions_total
            .with_label_values(&[program_name])
            .inc();

        if !success {
            self.counters
                .failed_transactions_total
                .with_label_values(&[program_name])
                .inc();
        }

        self.histograms
            .transaction_amounts
            .with_label_values(&[program_name])
            .observe(amount);
    }

    /// Record rule evaluation.
    pub fn record_rule_evaluation(&self, rule_name: &str, duration: Duration, triggered: bool) {
        self.counters
            .rule_evaluations_total
            .with_label_values(&[rule_name, if triggered { "triggered" } else { "passed" }])
            .inc();

        self.histograms
            .rule_evaluation_duration
            .with_label_values(&[rule_name])
            .observe(duration.as_secs_f64());
    }

    /// Update total value locked for a program.
    pub fn update_tvl(&self, program_name: &str, tvl: f64) {
        self.gauges
            .total_value_locked
            .with_label_values(&[program_name])
            .set(tvl);

        // Also add to sliding window
        self.add_to_window(&format!("{}_tvl", program_name), tvl);
    }

    /// Update token price.
    pub fn update_token_price(&self, token_symbol: &str, price: f64) {
        self.gauges
            .token_prices
            .with_label_values(&[token_symbol])
            .set(price);

        // Add to sliding window for trend analysis
        self.add_to_window(&format!("{}_price", token_symbol), price);
    }

    /// Update failure rate for a program.
    pub fn update_failure_rate(&self, program_name: &str, rate: f64) {
        self.gauges
            .failure_rate
            .with_label_values(&[program_name])
            .set(rate);

        self.add_to_window(&format!("{}_failure_rate", program_name), rate);
    }

    /// Record event processing time.
    pub fn record_event_processing_time(&self, duration_seconds: f64) {
        self.histograms
            .event_processing_latency
            .observe(duration_seconds);
    }

    /// Add a value to a sliding window.
    pub fn add_to_window(&self, metric_name: &str, value: f64) {
        let mut window = self
            .windows
            .entry(metric_name.to_string())
            .or_insert_with(|| SlidingWindow::new(Duration::from_secs(3600), 1000)); // 1 hour window

        window.add(value);
    }

    /// Set a custom metric value.
    pub fn set_custom_metric(&self, name: &str, value: MetricValue) {
        self.custom_metrics.insert(name.to_string(), value);
    }

    /// Get a metrics snapshot for rule evaluation.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let mut values = HashMap::new();
        let mut windows = HashMap::new();

        // For now, collect basic metrics from our known metric types
        // This is a simplified approach that avoids accessing private prometheus fields

        // Collect custom metrics
        for entry in self.custom_metrics.iter() {
            let value = match entry.value() {
                MetricValue::Counter(v) | MetricValue::Gauge(v) => *v,
                MetricValue::Histogram { sum, .. } => *sum,
            };
            values.insert(entry.key().clone(), value);
        }

        // Collect sliding window statistics
        for entry in self.windows.iter() {
            if let Some(stats) = entry.value().stats() {
                windows.insert(entry.key().clone(), stats);
            }
        }

        MetricsSnapshot {
            timestamp: Utc::now(),
            values,
            windows,
        }
    }

    /// Get Prometheus registry for HTTP endpoint.
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }

    /// Export metrics in Prometheus format.
    pub fn export(&self) -> String {
        prometheus::TextEncoder::new()
            .encode_to_string(&self.registry.gather())
            .unwrap_or_default()
    }
}

impl MetricsCounters {
    fn new(registry: &Registry) -> MetricsResult<Self> {
        let events_total = IntCounterVec::new(
            prometheus::Opts::new("watchtower_events_total", "Total events processed"),
            &["program", "event_type"],
        )?;
        registry.register(Box::new(events_total.clone()))?;

        let alerts_total = IntCounterVec::new(
            prometheus::Opts::new("watchtower_alerts_total", "Total alerts generated"),
            &["rule", "severity"],
        )?;
        registry.register(Box::new(alerts_total.clone()))?;

        let transactions_total = IntCounterVec::new(
            prometheus::Opts::new(
                "watchtower_transactions_total",
                "Total transactions processed",
            ),
            &["program"],
        )?;
        registry.register(Box::new(transactions_total.clone()))?;

        let failed_transactions_total = IntCounterVec::new(
            prometheus::Opts::new(
                "watchtower_failed_transactions_total",
                "Total failed transactions",
            ),
            &["program"],
        )?;
        registry.register(Box::new(failed_transactions_total.clone()))?;

        let rule_evaluations_total = IntCounterVec::new(
            prometheus::Opts::new(
                "watchtower_rule_evaluations_total",
                "Total rule evaluations",
            ),
            &["rule", "result"],
        )?;
        registry.register(Box::new(rule_evaluations_total.clone()))?;

        Ok(Self {
            events_total,
            alerts_total,
            transactions_total,
            failed_transactions_total,
            rule_evaluations_total,
        })
    }
}

impl MetricsGauges {
    fn new(registry: &Registry) -> MetricsResult<Self> {
        let active_connections = IntGauge::new(
            "watchtower_active_connections",
            "Number of active WebSocket connections",
        )?;
        registry.register(Box::new(active_connections.clone()))?;

        let total_value_locked = GaugeVec::new(
            prometheus::Opts::new(
                "watchtower_total_value_locked",
                "Total value locked in programs",
            ),
            &["program"],
        )?;
        registry.register(Box::new(total_value_locked.clone()))?;

        let token_prices = GaugeVec::new(
            prometheus::Opts::new("watchtower_token_prices", "Current token prices"),
            &["token"],
        )?;
        registry.register(Box::new(token_prices.clone()))?;

        let program_accounts = IntGaugeVec::new(
            prometheus::Opts::new("watchtower_program_accounts", "Number of program accounts"),
            &["program"],
        )?;
        registry.register(Box::new(program_accounts.clone()))?;

        let failure_rate = GaugeVec::new(
            prometheus::Opts::new("watchtower_failure_rate", "Transaction failure rate"),
            &["program"],
        )?;
        registry.register(Box::new(failure_rate.clone()))?;

        Ok(Self {
            active_connections,
            total_value_locked,
            token_prices,
            program_accounts,
            failure_rate,
        })
    }
}

impl MetricsHistograms {
    fn new(registry: &Registry) -> MetricsResult<Self> {
        let transaction_amounts = HistogramVec::new(
            prometheus::HistogramOpts::new("watchtower_transaction_amounts", "Transaction amounts")
                .buckets(vec![
                    100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0,
                ]),
            &["program"],
        )?;
        registry.register(Box::new(transaction_amounts.clone()))?;

        let rule_evaluation_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "watchtower_rule_evaluation_duration_seconds",
                "Rule evaluation duration",
            )
            .buckets(prometheus::DEFAULT_BUCKETS.to_vec()),
            &["rule"],
        )?;
        registry.register(Box::new(rule_evaluation_duration.clone()))?;

        let event_processing_latency = Histogram::with_opts(
            prometheus::HistogramOpts::new(
                "watchtower_event_processing_latency_seconds",
                "Event processing latency",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
        )?;
        registry.register(Box::new(event_processing_latency.clone()))?;

        Ok(Self {
            transaction_amounts,
            rule_evaluation_duration,
            event_processing_latency,
        })
    }
}

impl SlidingWindow {
    pub fn new(duration: Duration, max_points: usize) -> Self {
        Self {
            duration,
            data: Vec::new(),
            max_points,
        }
    }

    pub fn add(&mut self, value: f64) {
        let now = Instant::now();
        self.data.push((now, value));

        // Remove old data points
        let cutoff = now - self.duration;
        self.data.retain(|(timestamp, _)| *timestamp > cutoff);

        // Limit number of points
        if self.data.len() > self.max_points {
            let excess = self.data.len() - self.max_points;
            self.data.drain(0..excess);
        }
    }

    pub fn stats(&self) -> Option<WindowStats> {
        if self.data.is_empty() {
            return None;
        }

        let values: Vec<f64> = self.data.iter().map(|(_, v)| *v).collect();
        let count = values.len();
        let sum: f64 = values.iter().sum();
        let avg = sum / count as f64;

        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        // Calculate standard deviation
        let variance: f64 = values.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / count as f64;
        let std_dev = variance.sqrt();

        // Calculate percentiles
        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut percentiles = HashMap::new();
        percentiles.insert("50th".to_string(), percentile(&sorted_values, 0.5));
        percentiles.insert("90th".to_string(), percentile(&sorted_values, 0.9));
        percentiles.insert("95th".to_string(), percentile(&sorted_values, 0.95));
        percentiles.insert("99th".to_string(), percentile(&sorted_values, 0.99));

        Some(WindowStats {
            avg,
            min,
            max,
            sum,
            count,
            std_dev,
            percentiles,
        })
    }
}

fn percentile(sorted_values: &[f64], p: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let index = (p * (sorted_values.len() - 1) as f64) as usize;
    sorted_values[index.min(sorted_values.len() - 1)]
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics collector")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert!(collector.is_ok());
    }

    #[test]
    fn test_sliding_window() {
        let mut window = SlidingWindow::new(Duration::from_secs(60), 100);

        window.add(10.0);
        window.add(20.0);
        window.add(30.0);

        let stats = window.stats().unwrap();
        assert_eq!(stats.count, 3);
        assert_eq!(stats.avg, 20.0);
        assert_eq!(stats.min, 10.0);
        assert_eq!(stats.max, 30.0);
    }

    #[test]
    fn test_percentile_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert_eq!(percentile(&values, 0.5), 5.0);
        assert_eq!(percentile(&values, 0.9), 9.0);
    }
}
