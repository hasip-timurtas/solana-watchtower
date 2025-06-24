//! Template engine for rendering notification messages.

use crate::{NotifierError, NotifierResult};
use serde_json::{json, Value};
use std::collections::HashMap;
use tera::{Context, Tera};
use watchtower_engine::Alert;

/// Template engine for rendering notification messages.
pub struct TemplateEngine {
    /// Tera template engine
    tera: Tera,
}

impl TemplateEngine {
    /// Create a new template engine.
    pub fn new() -> Self {
        let mut tera = Tera::default();
        
        // Add built-in templates
        tera.add_raw_templates(vec![
            ("email_default", include_str!("../templates/email_default.html")),
            ("email_batch", include_str!("../templates/email_batch.html")),
            ("telegram_default", include_str!("../templates/telegram_default.md")),
            ("slack_default", include_str!("../templates/slack_default.txt")),
            ("discord_default", include_str!("../templates/discord_default.txt")),
        ]).unwrap_or_else(|e| {
            tracing::warn!("Failed to load built-in templates: {}", e);
        });

        Self { tera }
    }

    /// Render a template with the given data.
    pub fn render_template(&self, template_str: &str, data: &HashMap<String, Value>) -> NotifierResult<String> {
        let context = Context::from_serialize(data)?;
        
        // Try to render as inline template first
        match self.tera.render_str(template_str, &context) {
            Ok(rendered) => Ok(rendered),
            Err(e) => Err(NotifierError::Template(e)),
        }
    }

    /// Render default email template for an alert.
    pub fn render_default_email_template(&self, alert: &Alert) -> NotifierResult<String> {
        let context = self.create_alert_context(alert)?;
        
        match self.tera.render("email_default", &context) {
            Ok(rendered) => Ok(rendered),
            Err(_) => {
                // Fallback to simple HTML template
                Ok(self.render_fallback_email_template(alert))
            }
        }
    }

    /// Render batch email template for multiple alerts.
    pub fn render_batch_email_template(&self, alerts: &[Alert]) -> NotifierResult<String> {
        let mut context = Context::new();
        context.insert("alerts", alerts);
        context.insert("alert_count", &alerts.len());
        context.insert("timestamp", &chrono::Utc::now().to_rfc3339());

        match self.tera.render("email_batch", &context) {
            Ok(rendered) => Ok(rendered),
            Err(_) => {
                // Fallback to simple HTML template
                Ok(self.render_fallback_batch_email_template(alerts))
            }
        }
    }

    /// Render default Telegram template for an alert.
    pub fn render_default_telegram_template(&self, alert: &Alert) -> NotifierResult<String> {
        let context = self.create_alert_context(alert)?;
        
        match self.tera.render("telegram_default", &context) {
            Ok(rendered) => Ok(rendered),
            Err(_) => {
                // Fallback to simple Markdown template
                Ok(self.render_fallback_telegram_template(alert))
            }
        }
    }

    /// Render default Slack template for an alert.
    pub fn render_default_slack_template(&self, alert: &Alert) -> NotifierResult<String> {
        let context = self.create_alert_context(alert)?;
        
        match self.tera.render("slack_default", &context) {
            Ok(rendered) => Ok(rendered),
            Err(_) => {
                // Fallback to simple text template
                Ok(self.render_fallback_slack_template(alert))
            }
        }
    }

    /// Render default Discord template for an alert.
    pub fn render_default_discord_template(&self, alert: &Alert) -> NotifierResult<String> {
        let context = self.create_alert_context(alert)?;
        
        match self.tera.render("discord_default", &context) {
            Ok(rendered) => Ok(rendered),
            Err(_) => {
                // Fallback to simple text template
                Ok(self.render_fallback_discord_template(alert))
            }
        }
    }

    /// Create template context from alert data.
    fn create_alert_context(&self, alert: &Alert) -> NotifierResult<Context> {
        let mut context = Context::new();
        
        context.insert("alert", alert);
        context.insert("alert_id", &alert.id);
        context.insert("rule_name", &alert.rule_name);
        context.insert("message", &alert.message);
        context.insert("severity", &alert.severity.as_str());
        context.insert("severity_upper", &alert.severity.as_str().to_uppercase());
        context.insert("program_id", &alert.program_id.to_string());
        context.insert("program_name", &alert.program_name);
        context.insert("confidence", &(alert.confidence * 100.0));
        context.insert("timestamp", &alert.timestamp.to_rfc3339());
        context.insert("timestamp_human", &alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string());
        context.insert("suggested_actions", &alert.suggested_actions);
        context.insert("metadata", &alert.metadata);
        
        // Add severity-specific styling
        let severity_color = match alert.severity {
            watchtower_engine::AlertSeverity::Critical => "#FF0000",
            watchtower_engine::AlertSeverity::High => "#FF8C00",
            watchtower_engine::AlertSeverity::Medium => "#FFD700",
            watchtower_engine::AlertSeverity::Low => "#32CD32",
            watchtower_engine::AlertSeverity::Info => "#87CEEB",
        };
        context.insert("severity_color", &severity_color);
        
        let severity_emoji = match alert.severity {
            watchtower_engine::AlertSeverity::Critical => "🔴",
            watchtower_engine::AlertSeverity::High => "🟠",
            watchtower_engine::AlertSeverity::Medium => "🟡",
            watchtower_engine::AlertSeverity::Low => "🟢",
            watchtower_engine::AlertSeverity::Info => "🔵",
        };
        context.insert("severity_emoji", &severity_emoji);
        
        Ok(context)
    }

    /// Fallback email template when Tera fails.
    fn render_fallback_email_template(&self, alert: &Alert) -> String {
        format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Solana Watchtower Alert</title>
                <style>
                    body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; }}
                    .container {{ max-width: 600px; margin: 0 auto; background-color: white; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
                    .header {{ background-color: {}; color: white; padding: 20px; border-radius: 8px 8px 0 0; }}
                    .content {{ padding: 20px; }}
                    .field {{ margin-bottom: 15px; }}
                    .label {{ font-weight: bold; color: #333; }}
                    .value {{ color: #666; }}
                    .actions {{ background-color: #f8f9fa; padding: 15px; border-radius: 4px; margin-top: 20px; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h1>🛡️ Solana Watchtower Alert</h1>
                        <h2>{} - {}</h2>
                    </div>
                    <div class="content">
                        <div class="field">
                            <span class="label">Rule:</span>
                            <span class="value">{}</span>
                        </div>
                        <div class="field">
                            <span class="label">Program:</span>
                            <span class="value">{}</span>
                        </div>
                        <div class="field">
                            <span class="label">Message:</span>
                            <span class="value">{}</span>
                        </div>
                        <div class="field">
                            <span class="label">Confidence:</span>
                            <span class="value">{:.1}%</span>
                        </div>
                        <div class="field">
                            <span class="label">Time:</span>
                            <span class="value">{}</span>
                        </div>
                        {}
                    </div>
                </div>
            </body>
            </html>
            "#,
            alert.severity.color(),
            alert.severity.as_str().to_uppercase(),
            alert.rule_name,
            alert.rule_name,
            alert.program_name,
            alert.message,
            alert.confidence * 100.0,
            alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            if !alert.suggested_actions.is_empty() {
                format!(
                    r#"<div class="actions">
                        <div class="label">Suggested Actions:</div>
                        <ul>{}</ul>
                    </div>"#,
                    alert.suggested_actions.iter()
                        .map(|action| format!("<li>{}</li>", action))
                        .collect::<Vec<_>>()
                        .join("")
                )
            } else {
                String::new()
            }
        )
    }

    /// Fallback batch email template.
    fn render_fallback_batch_email_template(&self, alerts: &[Alert]) -> String {
        let alerts_html = alerts.iter()
            .map(|alert| {
                format!(
                    r#"
                    <div style="border: 1px solid #ddd; border-radius: 4px; padding: 15px; margin-bottom: 15px;">
                        <h3 style="margin: 0 0 10px 0; color: {};">{} - {}</h3>
                        <p><strong>Program:</strong> {}</p>
                        <p><strong>Message:</strong> {}</p>
                        <p><strong>Time:</strong> {}</p>
                    </div>
                    "#,
                    alert.severity.color(),
                    alert.severity.as_str().to_uppercase(),
                    alert.rule_name,
                    alert.program_name,
                    alert.message,
                    alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                )
            })
            .collect::<Vec<_>>()
            .join("");

        format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Solana Watchtower - {} Alerts</title>
                <style>
                    body {{ font-family: Arial, sans-serif; margin: 0; padding: 20px; background-color: #f5f5f5; }}
                    .container {{ max-width: 800px; margin: 0 auto; background-color: white; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
                    .header {{ background-color: #007bff; color: white; padding: 20px; border-radius: 8px 8px 0 0; }}
                    .content {{ padding: 20px; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <div class="header">
                        <h1>🛡️ Solana Watchtower Alert Summary</h1>
                        <h2>{} New Alerts</h2>
                    </div>
                    <div class="content">
                        {}
                    </div>
                </div>
            </body>
            </html>
            "#,
            alerts.len(),
            alerts.len(),
            alerts_html
        )
    }

    /// Fallback Telegram template.
    fn render_fallback_telegram_template(&self, alert: &Alert) -> String {
        let emoji = match alert.severity {
            watchtower_engine::AlertSeverity::Critical => "🔴",
            watchtower_engine::AlertSeverity::High => "🟠",
            watchtower_engine::AlertSeverity::Medium => "🟡",
            watchtower_engine::AlertSeverity::Low => "🟢",
            watchtower_engine::AlertSeverity::Info => "🔵",
        };

        let mut message = format!(
            r#"{} *Solana Watchtower Alert*

*Severity:* {}
*Rule:* `{}`
*Program:* `{}`
*Message:* {}
*Confidence:* {:.1}%
*Time:* {}"#,
            emoji,
            alert.severity.as_str().to_uppercase(),
            alert.rule_name,
            alert.program_name,
            alert.message,
            alert.confidence * 100.0,
            alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );

        if !alert.suggested_actions.is_empty() {
            message.push_str("\n\n*Suggested Actions:*");
            for action in &alert.suggested_actions {
                message.push_str(&format!("\n• {}", action));
            }
        }

        message
    }

    /// Fallback Slack template.
    fn render_fallback_slack_template(&self, alert: &Alert) -> String {
        format!(
            "🛡️ *Solana Watchtower Alert*\n\n*Severity:* {}\n*Rule:* {}\n*Program:* {}\n*Message:* {}\n*Confidence:* {:.1}%\n*Time:* {}",
            alert.severity.as_str().to_uppercase(),
            alert.rule_name,
            alert.program_name,
            alert.message,
            alert.confidence * 100.0,
            alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }

    /// Fallback Discord template.
    fn render_fallback_discord_template(&self, alert: &Alert) -> String {
        let emoji = match alert.severity {
            watchtower_engine::AlertSeverity::Critical => "🔴",
            watchtower_engine::AlertSeverity::High => "🟠",
            watchtower_engine::AlertSeverity::Medium => "🟡",
            watchtower_engine::AlertSeverity::Low => "🟢",
            watchtower_engine::AlertSeverity::Info => "🔵",
        };

        format!(
            "{} **Solana Watchtower Alert**\n\n**Severity:** {}\n**Rule:** {}\n**Program:** {}\n**Message:** {}\n**Confidence:** {:.1}%\n**Time:** {}",
            emoji,
            alert.severity.as_str().to_uppercase(),
            alert.rule_name,
            alert.program_name,
            alert.message,
            alert.confidence * 100.0,
            alert.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
} 