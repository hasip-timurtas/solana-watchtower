//! Event filtering and subscription management for Solana program monitoring.

use crate::{config::ProgramConfig, events::ProgramEvent, SubscriberResult};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::collections::HashSet;

/// Event filter that determines which events should be processed.
pub struct EventFilter {
    /// Programs to monitor
    monitored_programs: HashSet<Pubkey>,
    
    /// Program configurations
    program_configs: Vec<ProgramConfig>,
    
    /// Whether to include failed transactions
    include_failed: bool,
    
    /// Whether to include vote transactions
    include_votes: bool,
}

impl EventFilter {
    /// Create a new event filter from program configurations.
    pub fn new(program_configs: Vec<ProgramConfig>, include_failed: bool, include_votes: bool) -> Self {
        let monitored_programs = program_configs.iter().map(|p| p.id).collect();
        
        Self {
            monitored_programs,
            program_configs,
            include_failed,
            include_votes,
        }
    }
    
    /// Check if a transaction should be processed based on the filter.
    pub fn should_process_transaction(
        &self,
        transaction: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> bool {
        // Check if transaction involves any monitored programs
        if !self.involves_monitored_program(transaction) {
            return false;
        }
        
        // Check if failed transactions should be included
        if !self.include_failed && transaction.transaction.meta.as_ref()
            .map(|meta| meta.err.is_some())
            .unwrap_or(false) {
            return false;
        }
        
        // Check if vote transactions should be included
        if !self.include_votes && self.is_vote_transaction(transaction) {
            return false;
        }
        
        true
    }
    
    /// Check if a program event should be processed.
    pub fn should_process_event(&self, event: &ProgramEvent) -> bool {
        self.monitored_programs.contains(&event.program_id)
    }
    
    /// Get the configuration for a specific program.
    pub fn get_program_config(&self, program_id: &Pubkey) -> Option<&ProgramConfig> {
        self.program_configs.iter().find(|p| &p.id == program_id)
    }
    
    /// Get all monitored program IDs.
    pub fn monitored_programs(&self) -> &HashSet<Pubkey> {
        &self.monitored_programs
    }
    
    /// Check if a transaction involves any monitored programs.
    fn involves_monitored_program(
        &self,
        transaction: &EncodedConfirmedTransactionWithStatusMeta,
    ) -> bool {
        // Check transaction accounts
        if let Some(account_keys) = transaction.transaction.transaction.decode() {
            if let Ok(decoded) = account_keys {
                for account in &decoded.message.account_keys {
                    if self.monitored_programs.contains(account) {
                        return true;
                    }
                }
            }
        }
        
        // Check program IDs in transaction meta
        if let Some(meta) = &transaction.transaction.meta {
            if let Some(loaded_addresses) = &meta.loaded_addresses {
                for account in &loaded_addresses.readonly {
                    if let Ok(pubkey) = account.parse::<Pubkey>() {
                        if self.monitored_programs.contains(&pubkey) {
                            return true;
                        }
                    }
                }
                for account in &loaded_addresses.writable {
                    if let Ok(pubkey) = account.parse::<Pubkey>() {
                        if self.monitored_programs.contains(&pubkey) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }
    
    /// Check if a transaction is a vote transaction.
    fn is_vote_transaction(&self, transaction: &EncodedConfirmedTransactionWithStatusMeta) -> bool {
        // Simple heuristic: check if the transaction involves the vote program
        const VOTE_PROGRAM_ID: &str = "Vote111111111111111111111111111111111111111";
        
        if let Some(account_keys) = transaction.transaction.transaction.decode() {
            if let Ok(decoded) = account_keys {
                for account in &decoded.message.account_keys {
                    if account.to_string() == VOTE_PROGRAM_ID {
                        return true;
                    }
                }
            }
        }
        
        false
    }
}

/// Subscription manager for WebSocket connections.
pub struct SubscriptionManager {
    /// Active subscriptions mapped by subscription ID
    active_subscriptions: std::collections::HashMap<u64, SubscriptionType>,
    
    /// Next subscription ID
    next_id: u64,
}

/// Types of subscriptions that can be managed.
#[derive(Debug, Clone)]
pub enum SubscriptionType {
    /// Account subscription
    Account { pubkey: Pubkey },
    
    /// Program subscription
    Program { program_id: Pubkey },
    
    /// Signature subscription
    Signature { signature: String },
    
    /// Slot subscription
    Slot,
    
    /// Root subscription
    Root,
    
    /// Logs subscription
    Logs { mentions: Vec<Pubkey> },
}

impl SubscriptionManager {
    /// Create a new subscription manager.
    pub fn new() -> Self {
        Self {
            active_subscriptions: std::collections::HashMap::new(),
            next_id: 1,
        }
    }
    
    /// Add a new subscription.
    pub fn add_subscription(&mut self, subscription_type: SubscriptionType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.active_subscriptions.insert(id, subscription_type);
        id
    }
    
    /// Remove a subscription.
    pub fn remove_subscription(&mut self, id: u64) -> Option<SubscriptionType> {
        self.active_subscriptions.remove(&id)
    }
    
    /// Get all active subscription IDs.
    pub fn active_subscription_ids(&self) -> Vec<u64> {
        self.active_subscriptions.keys().copied().collect()
    }
    
    /// Get a subscription by ID.
    pub fn get_subscription(&self, id: u64) -> Option<&SubscriptionType> {
        self.active_subscriptions.get(&id)
    }
    
    /// Clear all subscriptions.
    pub fn clear(&mut self) {
        self.active_subscriptions.clear();
    }
    
    /// Get the count of active subscriptions.
    pub fn count(&self) -> usize {
        self.active_subscriptions.len()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SubscriptionType {
    /// Get a human-readable description of the subscription.
    pub fn description(&self) -> String {
        match self {
            SubscriptionType::Account { pubkey } => format!("Account: {}", pubkey),
            SubscriptionType::Program { program_id } => format!("Program: {}", program_id),
            SubscriptionType::Signature { signature } => format!("Signature: {}", signature),
            SubscriptionType::Slot => "Slot updates".to_string(),
            SubscriptionType::Root => "Root updates".to_string(),
            SubscriptionType::Logs { mentions } => {
                format!("Logs mentioning {} programs", mentions.len())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProgramConfig;
    use solana_sdk::pubkey::Pubkey;
    
    #[test]
    fn test_event_filter_creation() {
        let program_id = Pubkey::new_unique();
        let config = ProgramConfig {
            id: program_id,
            name: "Test Program".to_string(),
            monitor_accounts: true,
            monitor_transactions: true,
            monitor_logs: true,
            instruction_filters: None,
        };
        
        let filter = EventFilter::new(vec![config], false, false);
        assert!(filter.monitored_programs.contains(&program_id));
        assert_eq!(filter.monitored_programs.len(), 1);
    }
    
    #[test]
    fn test_subscription_manager() {
        let mut manager = SubscriptionManager::new();
        assert_eq!(manager.count(), 0);
        
        let program_id = Pubkey::new_unique();
        let subscription = SubscriptionType::Program { program_id };
        let id = manager.add_subscription(subscription);
        
        assert_eq!(manager.count(), 1);
        assert!(manager.get_subscription(id).is_some());
        
        let removed = manager.remove_subscription(id);
        assert!(removed.is_some());
        assert_eq!(manager.count(), 0);
    }
} 