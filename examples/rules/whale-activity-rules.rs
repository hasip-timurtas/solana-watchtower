// Whale Activity Monitoring Rules
// Custom rules for detecting large holder movements and whale activities

use watchtower_engine::rules::{Rule, RuleContext, RuleResult};
use watchtower_subscriber::events::ProgramEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhaleTransactionRule {
    pub name: String,
    pub threshold: u64,  // Minimum transaction amount to be considered "whale"
    pub whale_accounts: Vec<String>,  // Known whale accounts to monitor
    pub window_minutes: u32,
    pub severity: String,
}

impl Rule for WhaleTransactionRule {
    fn evaluate(&self, context: &RuleContext) -> RuleResult {
        let events = context.get_events_in_window(self.window_minutes);
        
        for event in &events {
            // Check if transaction amount exceeds whale threshold
            if let Some(amount) = event.transaction_amount {
                if amount > self.threshold {
                    return RuleResult::Alert {
                        severity: self.severity.clone(),
                        message: format!(
                            "Large transaction detected: {} SOL from signature {}",
                            amount / 1_000_000_000,
                            event.signature
                        ),
                        metadata: vec![
                            ("amount_lamports".to_string(), amount.to_string()),
                            ("amount_sol".to_string(), (amount / 1_000_000_000).to_string()),
                            ("signature".to_string(), event.signature.clone()),
                            ("block_time".to_string(), event.block_time.to_string()),
                        ],
                    };
                }
            }
            
            // Check if any whale accounts are involved
            for whale_account in &self.whale_accounts {
                if event.accounts.contains(whale_account) {
                    return RuleResult::Alert {
                        severity: "medium".to_string(),
                        message: format!(
                            "Whale account activity detected: {}",
                            whale_account
                        ),
                        metadata: vec![
                            ("whale_account".to_string(), whale_account.clone()),
                            ("signature".to_string(), event.signature.clone()),
                            ("program".to_string(), event.program_id.clone()),
                        ],
                    };
                }
            }
        }
        
        RuleResult::Pass
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcentrationRiskRule {
    pub name: String,
    pub concentration_threshold: f64,  // Max % of total supply held by top accounts
    pub top_holder_count: usize,       // Number of top holders to check
    pub check_interval_hours: u32,     // How often to check concentration
}

impl Rule for ConcentrationRiskRule {
    fn evaluate(&self, context: &RuleContext) -> RuleResult {
        // This would analyze token holder distribution
        // and alert if concentration becomes too high
        
        let current_concentration = self.calculate_concentration(context);
        
        if current_concentration > self.concentration_threshold {
            RuleResult::Alert {
                severity: "high".to_string(),
                message: format!(
                    "High concentration risk: top {} holders control {:.1}% of supply",
                    self.top_holder_count,
                    current_concentration * 100.0
                ),
                metadata: vec![
                    ("concentration_percent".to_string(), (current_concentration * 100.0).to_string()),
                    ("threshold_percent".to_string(), (self.concentration_threshold * 100.0).to_string()),
                    ("top_holders".to_string(), self.top_holder_count.to_string()),
                ],
            }
        } else {
            RuleResult::Pass
        }
    }
}

impl ConcentrationRiskRule {
    fn calculate_concentration(&self, context: &RuleContext) -> f64 {
        // This would analyze current token distribution
        // Placeholder implementation
        0.35 // 35% concentration
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuddenMovementRule {
    pub name: String,
    pub movement_threshold: f64,  // % of holdings moved in timeframe
    pub timeframe_minutes: u32,
    pub min_account_value: u64,   // Minimum account value to monitor
}

impl Rule for SuddenMovementRule {
    fn evaluate(&self, context: &RuleContext) -> RuleResult {
        let events = context.get_events_in_window(self.timeframe_minutes);
        let account_movements = self.analyze_account_movements(&events);
        
        for (account, movement_percent) in account_movements {
            if movement_percent > self.movement_threshold {
                return RuleResult::Alert {
                    severity: "medium".to_string(),
                    message: format!(
                        "Large position movement: {:.1}% moved from account {}",
                        movement_percent * 100.0,
                        account
                    ),
                    metadata: vec![
                        ("account".to_string(), account),
                        ("movement_percent".to_string(), (movement_percent * 100.0).to_string()),
                        ("timeframe_minutes".to_string(), self.timeframe_minutes.to_string()),
                    ],
                };
            }
        }
        
        RuleResult::Pass
    }
}

impl SuddenMovementRule {
    fn analyze_account_movements(&self, events: &[ProgramEvent]) -> HashMap<String, f64> {
        let mut movements = HashMap::new();
        
        // Analyze token movements for each account
        for event in events {
            // This would parse transfer instructions and calculate
            // what percentage of an account's holdings were moved
            // Placeholder implementation
            if let Some(amount) = event.transaction_amount {
                if amount > self.min_account_value {
                    for account in &event.accounts {
                        movements.insert(account.clone(), 0.25); // 25% movement
                    }
                }
            }
        }
        
        movements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_whale_transaction_rule() {
        let rule = WhaleTransactionRule {
            name: "whale-tracker".to_string(),
            threshold: 10_000_000_000_000, // 10,000 SOL
            whale_accounts: vec!["whale1".to_string(), "whale2".to_string()],
            window_minutes: 60,
            severity: "high".to_string(),
        };
        
        assert_eq!(rule.name, "whale-tracker");
        assert_eq!(rule.whale_accounts.len(), 2);
    }
} 