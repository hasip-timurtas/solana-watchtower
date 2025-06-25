use crate::{
    templates::{AlertsTemplate, IndexTemplate, MetricsTemplate, RulesTemplate, SettingsTemplate},
    websocket::handle_websocket,
    ApiResponse, AppState, DashboardError, DashboardResult, PaginationInfo, PaginationQuery,
};
use askama::Template;
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

// Helper function to format duration
fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    
    if total_seconds < 0 {
        return "0s".to_string();
    }
    
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    
    if days > 0 {
        format!("{}d {}h {}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Dashboard index page
pub async fn index(State(state): State<AppState>) -> DashboardResult<Html<String>> {
    let engine_state = state.engine.state().await;
    let alert_stats = state.alert_manager.statistics().await;
    let active_rules = state.engine.list_rules().await.len();

    let uptime_duration = chrono::Utc::now() - engine_state.start_time;
    let uptime_formatted = format_duration(uptime_duration);

    let template = IndexTemplate {
        title: "Solana Watchtower Dashboard".to_string(),
        engine_status: if engine_state.running {
            "Running".to_string()
        } else {
            "Stopped".to_string()
        },
        alert_count: alert_stats.total_alerts as usize,
        active_rules,
        uptime: uptime_formatted,
    };

    let html = template.render().map_err(DashboardError::Template)?;
    Ok(Html(html))
}

/// Alerts management page
pub async fn alerts_page(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> DashboardResult<Html<String>> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    let all_alerts = state.alert_manager.list_alerts(None).await;
    let total_alerts = all_alerts.len();

    // Simple pagination
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(total_alerts);
    let alerts = if start < total_alerts {
        all_alerts[start..end].to_vec()
    } else {
        Vec::new()
    };

    let template = AlertsTemplate {
        title: "Alerts".to_string(),
        alerts: alerts
            .into_iter()
            .map(|alert| AlertInfo {
                id: alert.id.clone(),
                severity: alert.severity.as_str().to_string(),
                message: alert.message.clone(),
                program_id: alert.program_id.to_string(),
                timestamp: alert.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                resolved: alert.resolved,
            })
            .collect(),
        pagination: PaginationInfo {
            page,
            limit,
            total: total_alerts as u32,
            pages: ((total_alerts as f64) / (limit as f64)).ceil() as u32,
        },
    };

    let html = template.render().map_err(DashboardError::Template)?;
    Ok(Html(html))
}

/// Metrics overview page
pub async fn metrics_page(State(state): State<AppState>) -> DashboardResult<Html<String>> {
    let metrics_snapshot = state.metrics.snapshot();

    // Convert metrics to display format
    let metric_items: Vec<MetricItem> = metrics_snapshot
        .values
        .into_iter()
        .map(|(name, value)| MetricItem {
            name,
            value: value.to_string(),
        })
        .collect();

    let template = MetricsTemplate {
        title: "System Metrics".to_string(),
        metrics: metric_items,
    };

    let html = template.render().map_err(DashboardError::Template)?;
    Ok(Html(html))
}

/// Rules management page
pub async fn rules_page(State(state): State<AppState>) -> DashboardResult<Html<String>> {
    let rule_names = state.engine.list_rules().await;

    let rule_items: Vec<RuleInfo> = rule_names
        .into_iter()
        .map(|name| RuleInfo {
            name: name.clone(),
            description: format!("Rule: {}", name),
            enabled: true,
            trigger_count: 0,
        })
        .collect();

    let template = RulesTemplate {
        title: "Monitoring Rules".to_string(),
        rules: rule_items,
    };

    let html = template.render().map_err(DashboardError::Template)?;
    Ok(Html(html))
}

/// Settings page
pub async fn settings_page(State(state): State<AppState>) -> DashboardResult<Html<String>> {
    let dashboard_state = state.dashboard_state.read().await;
    
    let template = SettingsTemplate {
        title: "Settings".to_string(),
        notification_channels: dashboard_state.notification_channels.clone(),
    };

    let html = template.render().map_err(DashboardError::Template)?;
    Ok(Html(html))
}

/// API: System status
pub async fn api_status(State(state): State<AppState>) -> Json<ApiResponse<SystemStatus>> {
    let engine_state = state.engine.state().await;
    let alert_stats = state.alert_manager.statistics().await;
    let active_rules = state.engine.list_rules().await.len();

    let status = SystemStatus {
        engine_status: if engine_state.running {
            "Running".to_string()
        } else {
            "Stopped".to_string()
        },
        alert_count: alert_stats.total_alerts as usize,
        active_rules,
        uptime_seconds: 8100, // TODO: Calculate actual uptime
        memory_usage_mb: 256, // TODO: Get actual memory usage
        connected_websockets: state.ws_connections.read().await.len(),
    };

    Json(ApiResponse::success(status))
}

/// API: Get alerts with pagination
pub async fn api_alerts(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Json<ApiResponse<Vec<AlertInfo>>> {
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    let all_alerts = state.alert_manager.list_alerts(None).await;
    let total_alerts = all_alerts.len();

    // Simple pagination
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(total_alerts);
    let alerts = if start < total_alerts {
        all_alerts[start..end].to_vec()
    } else {
        Vec::new()
    };

    let alert_infos: Vec<AlertInfo> = alerts
        .into_iter()
        .map(|alert| AlertInfo {
            id: alert.id.clone(),
            severity: alert.severity.as_str().to_string(),
            message: alert.message.clone(),
            program_id: alert.program_id.to_string(),
            timestamp: alert.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            resolved: alert.resolved,
        })
        .collect();

    let pagination = PaginationInfo {
        page,
        limit,
        total: total_alerts as u32,
        pages: ((total_alerts as f64) / (limit as f64)).ceil() as u32,
    };

    Json(ApiResponse::success_with_pagination(
        alert_infos,
        pagination,
    ))
}

/// API: Get specific alert details
pub async fn api_alert_detail(
    State(state): State<AppState>,
    Path(alert_id): Path<String>,
) -> Json<ApiResponse<AlertDetail>> {
    match state.alert_manager.get_alert(&alert_id) {
        Some(alert) => {
            let detail = AlertDetail {
                id: alert.id.clone(),
                severity: alert.severity.as_str().to_string(),
                message: alert.message.clone(),
                program_id: alert.program_id.to_string(),
                timestamp: alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                resolved: alert.resolved,
                metadata: alert
                    .metadata
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_string()))
                    .collect(),
                rule_name: alert.rule_name.clone(),
            };
            Json(ApiResponse::success(detail))
        }
        None => Json(ApiResponse::error("Alert not found")),
    }
}

/// API: Get metrics in JSON format
pub async fn api_metrics(State(state): State<AppState>) -> Json<ApiResponse<MetricsData>> {
    let metrics_snapshot = state.metrics.snapshot();

    let metrics_data = MetricsData {
        raw_prometheus: "# Prometheus metrics placeholder".to_string(),
        parsed_metrics: metrics_snapshot.values,
        timestamp: chrono::Utc::now().timestamp(),
    };

    Json(ApiResponse::success(metrics_data))
}

/// API: Get rules information
pub async fn api_rules(State(state): State<AppState>) -> Json<ApiResponse<Vec<RuleInfo>>> {
    let rule_names = state.engine.list_rules().await;

    let rule_infos: Vec<RuleInfo> = rule_names
        .into_iter()
        .map(|name| RuleInfo {
            name: name.clone(),
            description: format!("Rule: {}", name),
            enabled: true,
            trigger_count: 0,
        })
        .collect();

    Json(ApiResponse::success(rule_infos))
}

/// API: Get specific rule details
pub async fn api_rule_detail(
    State(state): State<AppState>,
    Path(rule_name): Path<String>,
) -> Json<ApiResponse<RuleDetail>> {
    let rule_names = state.engine.list_rules().await;

    if rule_names.contains(&rule_name) {
        let detail = RuleDetail {
            name: rule_name.clone(),
            description: format!("Rule: {}", rule_name),
            enabled: true,
            trigger_count: 0,
            last_triggered: None,
            configuration: HashMap::new(),
        };
        Json(ApiResponse::success(detail))
    } else {
        Json(ApiResponse::error("Rule not found"))
    }
}

/// API: Get monitored programs
pub async fn api_programs(State(_state): State<AppState>) -> Json<ApiResponse<Vec<ProgramInfo>>> {
    // TODO: Implement once get_monitored_programs is available
    let program_infos: Vec<ProgramInfo> = vec![ProgramInfo {
        id: "11111111111111111111111111111112".to_string(),
        name: "System Program".to_string(),
        events_processed: 1000,
        alerts_generated: 5,
        last_activity: Some("2024-01-15 10:30:00 UTC".to_string()),
    }];

    Json(ApiResponse::success(program_infos))
}

/// API: Get configuration
pub async fn api_config(State(state): State<AppState>) -> Json<ApiResponse<ConfigInfo>> {
    let dashboard_state = state.dashboard_state.read().await;
    
    let config = ConfigInfo {
        notification_channels: dashboard_state.notification_channels.clone(),
        monitoring_settings: dashboard_state.monitoring_settings.clone(),
    };

    Json(ApiResponse::success(config))
}

/// API: Update configuration
pub async fn api_update_config(
    State(state): State<AppState>,
    Json(config): Json<ConfigUpdateRequest>,
) -> Json<ApiResponse<String>> {
    info!("Configuration update requested: {:?}", config);
    
    let mut dashboard_state = state.dashboard_state.write().await;
    
    // Update notification channels if provided
    if let Some(channels) = config.notification_channels {
        dashboard_state.notification_channels = channels;
    }
    
    // Update monitoring settings if provided
    if let Some(settings) = config.monitoring_settings {
        dashboard_state.monitoring_settings = settings;
    }
    
    info!("Configuration updated successfully");
    Json(ApiResponse::success(
        "Configuration updated successfully".to_string(),
    ))
}

/// WebSocket handler
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

/// Health check endpoint
pub async fn health_check() -> Json<ApiResponse<HealthStatus>> {
    let status = HealthStatus {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };
    Json(ApiResponse::success(status))
}

/// Serve static files (embedded or from filesystem)
pub async fn serve_static(Path(file_path): Path<String>) -> Result<Response, StatusCode> {
    // For demo purposes, return a simple CSS file
    if file_path.ends_with(".css") {
        let css_content = include_str!("../static/style.css");
        Ok(([(header::CONTENT_TYPE, "text/css")], css_content).into_response())
    } else if file_path.ends_with(".js") {
        let js_content = include_str!("../static/app.js");
        Ok((
            [(header::CONTENT_TYPE, "application/javascript")],
            js_content,
        )
            .into_response())
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

// Data structures for API responses

#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub engine_status: String,
    pub alert_count: usize,
    pub active_rules: usize,
    pub uptime_seconds: u64,
    pub memory_usage_mb: u64,
    pub connected_websockets: usize,
}

#[derive(Debug, Serialize)]
pub struct AlertInfo {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub program_id: String,
    pub timestamp: String,
    pub resolved: bool,
}

#[derive(Debug, Serialize)]
pub struct AlertDetail {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub program_id: String,
    pub timestamp: String,
    pub resolved: bool,
    pub metadata: HashMap<String, String>,
    pub rule_name: String,
}

#[derive(Debug, Serialize)]
pub struct MetricItem {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct MetricsData {
    pub raw_prometheus: String,
    pub parsed_metrics: HashMap<String, f64>,
    pub timestamp: i64,
}

#[derive(Debug, Serialize)]
pub struct RuleInfo {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub trigger_count: u64,
}

#[derive(Debug, Serialize)]
pub struct RuleDetail {
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub trigger_count: u64,
    pub last_triggered: Option<String>,
    pub configuration: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct ProgramInfo {
    pub id: String,
    pub name: String,
    pub events_processed: u64,
    pub alerts_generated: u64,
    pub last_activity: Option<String>,
}

// Re-export types from lib.rs for convenience
pub use crate::{MonitoringSettings, NotificationChannel};

#[derive(Debug, Serialize)]
pub struct ConfigInfo {
    pub notification_channels: Vec<NotificationChannel>,
    pub monitoring_settings: MonitoringSettings,
}

#[derive(Debug, Deserialize)]
pub struct ConfigUpdateRequest {
    pub notification_channels: Option<Vec<NotificationChannel>>,
    pub monitoring_settings: Option<MonitoringSettings>,
}

#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub timestamp: i64,
}
