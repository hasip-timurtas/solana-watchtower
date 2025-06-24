use crate::handlers::{AlertInfo, MetricItem, NotificationChannel, RuleInfo};
use crate::PaginationInfo;
use askama::Template;

/// Base template for common layout
#[derive(Template)]
#[template(path = "base.html")]
pub struct BaseTemplate {
    pub title: String,
}

/// Dashboard index page template
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub title: String,
    pub engine_status: String,
    pub alert_count: usize,
    pub active_rules: usize,
    pub uptime: String,
}

/// Alerts page template
#[derive(Template)]
#[template(path = "alerts.html")]
pub struct AlertsTemplate {
    pub title: String,
    pub alerts: Vec<AlertInfo>,
    pub pagination: PaginationInfo,
}

/// Metrics page template
#[derive(Template)]
#[template(path = "metrics.html")]
pub struct MetricsTemplate {
    pub title: String,
    pub metrics: Vec<MetricItem>,
}

/// Rules page template
#[derive(Template)]
#[template(path = "rules.html")]
pub struct RulesTemplate {
    pub title: String,
    pub rules: Vec<RuleInfo>,
}

/// Settings page template
#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    pub title: String,
    pub notification_channels: Vec<NotificationChannel>,
}
