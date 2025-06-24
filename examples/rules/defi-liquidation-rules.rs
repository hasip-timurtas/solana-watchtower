// DeFi Liquidation Monitoring Rules
// Custom rules for detecting and alerting on liquidation events

use watchtower_engine::rules::{Rule, RuleContext, RuleResult};
use watchtower_subscriber::events::ProgramEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationVolumeRule {
    pub name: String,
    pub threshold: u64,  // Liquidation volume threshold in lamports
    pub window_minutes: u32,
    pub severity: String,
}

impl Rule for LiquidationVolumeRule {
    fn evaluate(&self, context: &RuleContext) -> RuleResult {
        let events = context.get_events_in_window(self.window_minutes);
        let total_liquidation_volume = self.calculate_liquidation_volume(&events);
        
        if total_liquidation_volume > self.threshold {
            RuleResult::Alert {
                severity: self.severity.clone(),
                message: format!(
                    "High liquidation volume detected: {} SOL in {} minutes",
                    total_liquidation_volume / 1_000_000_000,
                    self.window_minutes
                ),
                metadata: vec![
                    ("volume_lamports".to_string(), total_liquidation_volume.to_string()),
                    ("threshold_lamports".to_string(), self.threshold.to_string()),
                    ("window_minutes".to_string(), self.window_minutes.to_string()),
                ],
            }
        } else {
            RuleResult::Pass
        }
    }
}

impl LiquidationVolumeRule {
    fn calculate_liquidation_volume(&self, events: &[ProgramEvent]) -> u64 {
        events.iter()
            .filter(|event| self.is_liquidation_event(event))
            .map(|event| self.extract_liquidation_amount(event))
            .sum()
    }
    
    fn is_liquidation_event(&self, event: &ProgramEvent) -> bool {
        // Check if the event is a liquidation based on instruction data
        if let Some(instruction_name) = &event.instruction_name {
            instruction_name.contains("liquidate") || instruction_name.contains("Liquidate")
        } else {
            false
        }
    }
    
    fn extract_liquidation_amount(&self, event: &ProgramEvent) -> u64 {
        // Extract liquidation amount from event data
        // This would parse the actual instruction data
        event.transaction_amount.unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthFactorRule {
    pub name: String,
    pub critical_threshold: f64,  // Health factor below which to alert
    pub warning_threshold: f64,
    pub tracked_accounts: Vec<String>,  // Specific accounts to monitor
}

impl Rule for HealthFactorRule {
    fn evaluate(&self, context: &RuleContext) -> RuleResult {
        let latest_events = context.get_latest_events(100);
        
        for account in &self.tracked_accounts {
            if let Some(health_factor) = self.get_health_factor(account, &latest_events) {
                if health_factor < self.critical_threshold {
                    return RuleResult::Alert {
                        severity: "critical".to_string(),
                        message: format!(
                            "Critical health factor for account {}: {:.3}",
                            account, health_factor
                        ),
                        metadata: vec![
                            ("account".to_string(), account.clone()),
                            ("health_factor".to_string(), health_factor.to_string()),
                            ("threshold".to_string(), self.critical_threshold.to_string()),
                        ],
                    };
                } else if health_factor < self.warning_threshold {
                    return RuleResult::Alert {
                        severity: "warning".to_string(),
                        message: format!(
                            "Low health factor for account {}: {:.3}",
                            account, health_factor
                        ),
                        metadata: vec![
                            ("account".to_string(), account.clone()),
                            ("health_factor".to_string(), health_factor.to_string()),
                            ("threshold".to_string(), self.warning_threshold.to_string()),
                        ],
                    };
                }
            }
        }
        
        RuleResult::Pass
    }
}

impl HealthFactorRule {
    fn get_health_factor(&self, account: &str, events: &[ProgramEvent]) -> Option<f64> {
        // Parse recent events for this account to calculate health factor
        // This would involve parsing lending protocol account data
        
        // Placeholder implementation
        for event in events {
            if event.accounts.contains(&account.to_string()) {
                // Extract health factor from account data
                // This would parse the actual account data structure
                return Some(1.5); // Placeholder
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_liquidation_volume_rule() {
        let rule = LiquidationVolumeRule {
            name: "test-liquidation".to_string(),
            threshold: 1_000_000_000_000, // 1000 SOL
            window_minutes: 5,
            severity: "high".to_string(),
        };
        
        // Test would create mock events and verify rule behavior
        assert_eq!(rule.name, "test-liquidation");
    }
} 