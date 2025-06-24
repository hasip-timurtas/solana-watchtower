use crate::{
    templates::{
        AlertsTemplate, IndexTemplate, MetricsTemplate, RulesTemplate, SettingsTemplate,
    },
    websocket::handle_websocket,
    ApiResponse, AppState, DashboardError, DashboardResult, PaginationInfo, PaginationQuery,
};
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

/// Dashboard index page
pub async fn index(State(state): State<AppState>) -> DashboardResult<Html<String>> {
    let engine_status = state.engine.status().await;
    let alert_count = state.alert_manager.alert_count().await;
    let active_rules = state.engine.list_rules().await.len();

    let template = IndexTemplate {
        title: "Solana Watchtower Dashboard".to_string(),
        engine_status: format!("{:?}", engine_status),
        alert_count,
        active_rules,
        uptime: "2h 15m".to_string(), // TODO: Calculate actual uptime
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

    let alerts = state.alert_manager.get_recent_alerts(limit as usize).await;
    let total_alerts = state.alert_manager.alert_count().await;

    let template = AlertsTemplate {
        title: "Alerts".to_string(),
        alerts: alerts.into_iter().map(|alert| AlertInfo {
            id: alert.id.clone(),
            severity: format!("{:?}", alert.severity),
            message: alert.message.clone(),
            program_id: alert.program_id.to_string(),
            timestamp: alert.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            resolved: alert.resolved,
        }).collect(),
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
    let metrics_data = state.metrics.export();
    
    // Parse basic metrics for display
    let mut metric_items = Vec::new();
    for line in metrics_data.lines() {
        if !line.starts_with('#') && !line.trim().is_empty() {
            if let Some(space_idx) = line.find(' ') {
                let name = &line[..space_idx];
                let value = &line[space_idx + 1..];
                metric_items.push(MetricItem {
                    name: name.to_string(),
                    value: value.to_string(),
                });
            }
        }
    }

    let template = MetricsTemplate {
        title: "System Metrics".to_string(),
        metrics: metric_items,
    };

    let html = template.render().map_err(DashboardError::Template)?;
    Ok(Html(html))
}

/// Rules management page
pub async fn rules_page(State(state): State<AppState>) -> DashboardResult<Html<String>> {
    let rules = state.engine.list_rules().await;
    
    let rule_items: Vec<RuleInfo> = rules.into_iter().map(|rule| RuleInfo {
        name: rule.name,
        description: rule.description,
        enabled: rule.enabled,
        trigger_count: rule.trigger_count,
    }).collect();

    let template = RulesTemplate {
        title: "Monitoring Rules".to_string(),
        rules: rule_items,
    };

    let html = template.render().map_err(DashboardError::Template)?;
    Ok(Html(html))
}

/// Settings page
pub async fn settings_page(State(_state): State<AppState>) -> DashboardResult<Html<String>> {
    let template = SettingsTemplate {
        title: "Settings".to_string(),
        notification_channels: vec![
            NotificationChannel {
                name: "Email".to_string(),
                enabled: true,
                status: "Active".to_string(),
            },
            NotificationChannel {
                name: "Telegram".to_string(),
                enabled: true,
                status: "Active".to_string(),
            },
            NotificationChannel {
                name: "Slack".to_string(),
                enabled: false,
                status: "Disabled".to_string(),
            },
        ],
    };

    let html = template.render().map_err(DashboardError::Template)?;
    Ok(Html(html))
}

/// API: System status
pub async fn api_status(State(state): State<AppState>) -> Json<ApiResponse<SystemStatus>> {
    let engine_status = state.engine.status().await;
    let alert_count = state.alert_manager.alert_count().await;
    let active_rules = state.engine.list_rules().await.len();

    let status = SystemStatus {
        engine_status: format!("{:?}", engine_status),
        alert_count,
        active_rules,
        uptime_seconds: 8100, // TODO: Calculate actual uptime
        memory_usage_mb: 256,  // TODO: Get actual memory usage
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

    let alerts = state.alert_manager.get_recent_alerts(limit as usize).await;
    let total_alerts = state.alert_manager.alert_count().await;

    let alert_infos: Vec<AlertInfo> = alerts.into_iter().map(|alert| AlertInfo {
        id: alert.id.clone(),
        severity: format!("{:?}", alert.severity),
        message: alert.message.clone(),
        program_id: alert.program_id.to_string(),
        timestamp: alert.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
        resolved: alert.resolved,
    }).collect();

    let pagination = PaginationInfo {
        page,
        limit,
        total: total_alerts as u32,
        pages: ((total_alerts as f64) / (limit as f64)).ceil() as u32,
    };

    Json(ApiResponse::success_with_pagination(alert_infos, pagination))
}

/// API: Get specific alert details
pub async fn api_alert_detail(
    State(state): State<AppState>,
    Path(alert_id): Path<String>,
) -> Json<ApiResponse<AlertDetail>> {
    match state.alert_manager.get_alert(&alert_id).await {
        Some(alert) => {
            let detail = AlertDetail {
                id: alert.id.clone(),
                severity: format!("{:?}", alert.severity),
                message: alert.message.clone(),
                program_id: alert.program_id.to_string(),
                timestamp: alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                resolved: alert.resolved,
                metadata: alert.metadata.clone(),
                rule_name: alert.rule_name.clone(),
            };
            Json(ApiResponse::success(detail))
        }
        None => Json(ApiResponse::error("Alert not found")),
    }
}

/// API: Get metrics in JSON format
pub async fn api_metrics(State(state): State<AppState>) -> Json<ApiResponse<MetricsData>> {
    let raw_metrics = state.metrics.export();
    
    let mut parsed_metrics = HashMap::new();
    for line in raw_metrics.lines() {
        if !line.starts_with('#') && !line.trim().is_empty() {
            if let Some(space_idx) = line.find(' ') {
                let name = &line[..space_idx];
                let value_str = &line[space_idx + 1..];
                if let Ok(value) = value_str.parse::<f64>() {
                    parsed_metrics.insert(name.to_string(), value);
                }
            }
        }
    }

    let metrics_data = MetricsData {
        raw_prometheus: raw_metrics,
        parsed_metrics,
        timestamp: chrono::Utc::now().timestamp(),
    };

    Json(ApiResponse::success(metrics_data))
}

/// API: Get rules information
pub async fn api_rules(State(state): State<AppState>) -> Json<ApiResponse<Vec<RuleInfo>>> {
    let rules = state.engine.list_rules().await;
    
    let rule_infos: Vec<RuleInfo> = rules.into_iter().map(|rule| RuleInfo {
        name: rule.name,
        description: rule.description,
        enabled: rule.enabled,
        trigger_count: rule.trigger_count,
    }).collect();

    Json(ApiResponse::success(rule_infos))
}

/// API: Get specific rule details
pub async fn api_rule_detail(
    State(state): State<AppState>,
    Path(rule_name): Path<String>,
) -> Json<ApiResponse<RuleDetail>> {
    let rules = state.engine.list_rules().await;
    
    if let Some(rule) = rules.iter().find(|r| r.name == rule_name) {
        let detail = RuleDetail {
            name: rule.name.clone(),
            description: rule.description.clone(),
            enabled: rule.enabled,
            trigger_count: rule.trigger_count,
            last_triggered: rule.last_triggered.map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
            configuration: rule.configuration.clone(),
        };
        Json(ApiResponse::success(detail))
    } else {
        Json(ApiResponse::error("Rule not found"))
    }
}

/// API: Get monitored programs
pub async fn api_programs(State(state): State<AppState>) -> Json<ApiResponse<Vec<ProgramInfo>>> {
    let programs = state.engine.get_monitored_programs().await;
    
    let program_infos: Vec<ProgramInfo> = programs.into_iter().map(|program| ProgramInfo {
        id: program.id.to_string(),
        name: program.name,
        events_processed: program.events_processed,
        alerts_generated: program.alerts_generated,
        last_activity: program.last_activity.map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
    }).collect();

    Json(ApiResponse::success(program_infos))
}

/// API: Get configuration
pub async fn api_config(State(_state): State<AppState>) -> Json<ApiResponse<ConfigInfo>> {
    // TODO: Load actual configuration
    let config = ConfigInfo {
        notification_channels: vec![
            NotificationChannel {
                name: "Email".to_string(),
                enabled: true,
                status: "Active".to_string(),
            },
            NotificationChannel {
                name: "Telegram".to_string(),
                enabled: true,
                status: "Active".to_string(),
            },
        ],
        monitoring_settings: MonitoringSettings {
            max_events_per_minute: 1000,
            alert_retention_days: 30,
            enable_real_time_alerts: true,
        },
    };

    Json(ApiResponse::success(config))
}

/// API: Update configuration
pub async fn api_update_config(
    State(_state): State<AppState>,
    Json(config): Json<ConfigUpdateRequest>,
) -> Json<ApiResponse<String>> {
    // TODO: Implement configuration updates
    info!("Configuration update requested: {:?}", config);
    Json(ApiResponse::success("Configuration updated successfully".to_string()))
}

/// WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
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
        Ok((
            [(header::CONTENT_TYPE, "text/css")],
            css_content,
        ).into_response())
    } else if file_path.ends_with(".js") {
        let js_content = include_str!("../static/app.js");
        Ok((
            [(header::CONTENT_TYPE, "application/javascript")],
            js_content,
        ).into_response())
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

#[derive(Debug, Serialize)]
pub struct NotificationChannel {
    pub name: String,
    pub enabled: bool,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct MonitoringSettings {
    pub max_events_per_minute: u32,
    pub alert_retention_days: u32,
    pub enable_real_time_alerts: bool,
}

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