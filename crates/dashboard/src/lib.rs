use anyhow::Result;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use tracing::info;
use watchtower_engine::{AlertManager, MetricsCollector, MonitoringEngine};

mod handlers;
mod templates;
mod websocket;

pub use handlers::*;
pub use templates::*;
pub use websocket::*;

/// Dashboard configuration
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    pub host: String,
    pub port: u16,
    pub enable_cors: bool,
    pub static_dir: Option<String>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            enable_cors: true,
            static_dir: None,
        }
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<MonitoringEngine>,
    pub alert_manager: Arc<AlertManager>,
    pub metrics: Arc<MetricsCollector>,
    pub ws_connections: Arc<tokio::sync::RwLock<HashMap<String, WebSocketConnection>>>,
}

/// Dashboard server
pub struct DashboardServer {
    config: DashboardConfig,
    state: AppState,
}

impl DashboardServer {
    /// Create a new dashboard server
    pub fn new(
        config: DashboardConfig,
        engine: Arc<MonitoringEngine>,
        alert_manager: Arc<AlertManager>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        let state = AppState {
            engine,
            alert_manager,
            metrics,
            ws_connections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        };

        Self { config, state }
    }

    /// Start the dashboard server
    pub async fn start(self) -> Result<()> {
        let app = self.create_router();

        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

        let listener = TcpListener::bind(&addr).await?;

        info!(
            "Dashboard server starting on http://{}:{}",
            self.config.host, self.config.port
        );

        // Start WebSocket heartbeat task
        let ws_connections = self.state.ws_connections.clone();
        tokio::spawn(async move {
            websocket_heartbeat_task(ws_connections).await;
        });

        // Start alert broadcasting task
        let alert_manager = self.state.alert_manager.clone();
        let ws_connections = self.state.ws_connections.clone();
        tokio::spawn(async move {
            alert_broadcast_task(alert_manager, ws_connections).await;
        });

        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Create the application router
    fn create_router(&self) -> Router {
        let mut app = Router::new()
            // Main pages
            .route("/", get(handlers::index))
            .route("/alerts", get(handlers::alerts_page))
            .route("/metrics", get(handlers::metrics_page))
            .route("/rules", get(handlers::rules_page))
            .route("/settings", get(handlers::settings_page))
            // API endpoints
            .route("/api/status", get(handlers::api_status))
            .route("/api/alerts", get(handlers::api_alerts))
            .route("/api/alerts/:id", get(handlers::api_alert_detail))
            .route("/api/metrics", get(handlers::api_metrics))
            .route("/api/rules", get(handlers::api_rules))
            .route("/api/rules/:name", get(handlers::api_rule_detail))
            .route("/api/programs", get(handlers::api_programs))
            .route("/api/config", get(handlers::api_config))
            .route("/api/config", post(handlers::api_update_config))
            // WebSocket endpoint
            .route("/ws", get(handlers::websocket_handler))
            // Health check
            .route("/health", get(handlers::health_check))
            // State
            .with_state(self.state.clone());

        // Add middleware
        if self.config.enable_cors {
            app = app.layer(CorsLayer::permissive());
        }

        // Add static file serving
        if let Some(static_dir) = &self.config.static_dir {
            app = app.nest_service(
                "/static",
                ServeDir::new(static_dir)
                    .fallback(ServeFile::new(format!("{}/index.html", static_dir))),
            );
        } else {
            // Serve embedded static files
            app = app.route("/static/*file", get(handlers::serve_static));
        }

        app
    }
}

/// Query parameters for pagination
#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub filter: Option<String>,
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(20),
            sort: None,
            filter: None,
        }
    }
}

/// Standard API response format
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub pagination: Option<PaginationInfo>,
}

#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u32,
    pub pages: u32,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            pagination: None,
        }
    }

    pub fn success_with_pagination(data: T, pagination: PaginationInfo) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            pagination: Some(pagination),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            pagination: None,
        }
    }
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> axum::response::Response {
        let status = if self.success {
            StatusCode::OK
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };

        (status, Json(self)).into_response()
    }
}

/// Error handling for the dashboard
#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    #[error("Template error: {0}")]
    Template(#[from] askama::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for DashboardError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            DashboardError::Template(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Template error"),
            DashboardError::Json(_) => (StatusCode::BAD_REQUEST, "Invalid JSON"),
            DashboardError::Http(ref msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            DashboardError::WebSocket(ref msg) => (StatusCode::BAD_REQUEST, msg.as_str()),
            DashboardError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error"),
        };

        let body = Json(ApiResponse::<()>::error(error_message.to_string()));
        (status, body).into_response()
    }
}

pub type DashboardResult<T> = Result<T, DashboardError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_config_default() {
        let config = DashboardConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.enable_cors);
        assert!(config.static_dir.is_none());
    }

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data");
        assert!(response.success);
        assert_eq!(response.data, Some("test data"));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response: ApiResponse<()> = ApiResponse::error("test error");
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("test error".to_string()));
    }
}
