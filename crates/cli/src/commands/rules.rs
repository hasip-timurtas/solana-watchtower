use anyhow::Result;
use console::style;
use watchtower_engine::{
    LiquidityDropRule, LargeTransactionRule, OracleDeviationRule, FailureRateRule
};

pub async fn rules_list_command() -> Result<()> {
    println!("{}", style("Available Monitoring Rules:").bold());
    println!("{}", "─".repeat(60));

    let rules = [
        ("liquidity_drop", "Liquidity Drop Detection", "Monitors for sudden drops in liquidity pools"),
        ("large_transaction", "Large Transaction Detection", "Flags unusually large transactions"),
        ("oracle_deviation", "Oracle Price Deviation", "Detects price manipulation attempts"),
        ("failure_rate", "High Failure Rate Detection", "Monitors transaction failure rates"),
    ];

    for (name, title, description) in rules {
        println!(
            "{} {}",
            style(format!("• {:20}", name)).cyan().bold(),
            style(title).white().bold()
        );
        println!("  {}", style(description).dim());
        println!();
    }

    println!("{}", style("Use 'watchtower rules info <rule_name>' for detailed information").dim());
    Ok(())
}

pub async fn rules_info_command(rule_name: String) -> Result<()> {
    match rule_name.as_str() {
        "liquidity_drop" => show_liquidity_drop_info(),
        "large_transaction" => show_large_transaction_info(),
        "oracle_deviation" => show_oracle_deviation_info(),
        "failure_rate" => show_failure_rate_info(),
        _ => {
            println!(
                "{} Unknown rule: {}",
                style("✗").red().bold(),
                style(&rule_name).red()
            );
            println!("Use 'watchtower rules list' to see available rules.");
            std::process::exit(1);
        }
    }
    Ok(())
}

pub async fn rules_test_command(rule_name: String) -> Result<()> {
    println!(
        "{} Testing rule: {}",
        style("Running test for").cyan(),
        style(&rule_name).bold()
    );
    println!();

    match rule_name.as_str() {
        "liquidity_drop" => test_liquidity_drop_rule().await,
        "large_transaction" => test_large_transaction_rule().await,
        "oracle_deviation" => test_oracle_deviation_rule().await,
        "failure_rate" => test_failure_rate_rule().await,
        _ => {
            println!(
                "{} Unknown rule: {}",
                style("✗").red().bold(),
                style(&rule_name).red()
            );
            std::process::exit(1);
        }
    }
}

fn show_liquidity_drop_info() {
    println!("{}", style("Liquidity Drop Rule").bold().cyan());
    println!("{}", "─".repeat(50));
    println!("{}", style("Description:").bold());
    println!("Monitors liquidity pools for sudden drops that might indicate");
    println!("rug pulls, exploits, or other malicious activities.");
    println!();
    println!("{}", style("Parameters:").bold());
    println!("• threshold_percentage: Minimum drop percentage to trigger (default: 10%)");
    println!("• time_window_seconds: Time window to analyze (default: 300s)");
    println!("• min_liquidity_value: Minimum liquidity value to monitor (default: 1M)");
    println!();
    println!("{}", style("Triggers when:").bold());
    println!("Liquidity drops by more than the threshold within the time window");
}

fn show_large_transaction_info() {
    println!("{}", style("Large Transaction Rule").bold().cyan());
    println!("{}", "─".repeat(50));
    println!("{}", style("Description:").bold());
    println!("Detects unusually large transactions that might indicate");
    println!("whale movements, exploits, or suspicious activity.");
    println!();
    println!("{}", style("Parameters:").bold());
    println!("• threshold_percentage: Percentage of TVL threshold (default: 1%)");
    println!("• min_value_lamports: Minimum transaction value (default: 500K lamports)");
    println!();
    println!("{}", style("Triggers when:").bold());
    println!("Transaction value exceeds threshold percentage of total value locked");
}

fn show_oracle_deviation_info() {
    println!("{}", style("Oracle Deviation Rule").bold().cyan());
    println!("{}", "─".repeat(50));
    println!("{}", style("Description:").bold());
    println!("Monitors price oracles for significant deviations that might");
    println!("indicate price manipulation or oracle attacks.");
    println!();
    println!("{}", style("Parameters:").bold());
    println!("• threshold_percentage: Price deviation threshold (default: 5%)");
    println!("• reference_oracle: Reference oracle for comparison");
    println!();
    println!("{}", style("Triggers when:").bold());
    println!("Price deviates more than threshold from reference oracle");
}

fn show_failure_rate_info() {
    println!("{}", style("Failure Rate Rule").bold().cyan());
    println!("{}", "─".repeat(50));
    println!("{}", style("Description:").bold());
    println!("Monitors transaction failure rates to detect potential");
    println!("attacks, congestion, or system issues.");
    println!();
    println!("{}", style("Parameters:").bold());
    println!("• threshold_percentage: Failure rate threshold (default: 25%)");
    println!("• min_transactions: Minimum transactions to analyze (default: 10)");
    println!("• time_window_seconds: Analysis time window (default: 300s)");
    println!();
    println!("{}", style("Triggers when:").bold());
    println!("Failure rate exceeds threshold over the time window");
}

async fn test_liquidity_drop_rule() -> Result<()> {
    use watchtower_engine::{SolanaEvent, TransactionEvent};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    let rule = LiquidityDropRule::new(10.0, 300, 1000000);
    
    // Create test event
    let test_event = SolanaEvent::Transaction(TransactionEvent {
        signature: "test_signature".to_string(),
        slot: 12345,
        block_time: Some(chrono::Utc::now().timestamp()),
        program_id: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
        accounts: vec![],
        instruction_data: vec![],
        success: true,
        error: None,
        fee: 5000,
        pre_balances: vec![1000000000],  // 1 SOL before
        post_balances: vec![900000000],  // 0.9 SOL after (10% drop)
        pre_token_balances: vec![],
        post_token_balances: vec![],
        log_messages: vec![],
        compute_units_consumed: Some(10000),
    });

    println!("{}", style("Creating test transaction with 10% liquidity drop...").dim());
    
    match rule.evaluate(&test_event).await {
        Ok(Some(alert)) => {
            println!("{} Rule triggered alert:", style("✓").green().bold());
            println!("  Severity: {:?}", alert.severity);
            println!("  Message: {}", alert.message);
            println!("  Metadata: {:?}", alert.metadata);
        }
        Ok(None) => {
            println!("{} Rule did not trigger (threshold not met)", style("ⓘ").blue());
        }
        Err(e) => {
            println!("{} Rule evaluation failed: {}", style("✗").red().bold(), e);
        }
    }

    Ok(())
}

async fn test_large_transaction_rule() -> Result<()> {
    use watchtower_engine::{SolanaEvent, TransactionEvent};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    let rule = LargeTransactionRule::new(1.0, 500000);
    
    let test_event = SolanaEvent::Transaction(TransactionEvent {
        signature: "test_signature_large".to_string(),
        slot: 12346,
        block_time: Some(chrono::Utc::now().timestamp()),
        program_id: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
        accounts: vec![],
        instruction_data: vec![],
        success: true,
        error: None,
        fee: 5000,
        pre_balances: vec![100_000_000_000],  // 100 SOL
        post_balances: vec![50_000_000_000],  // 50 SOL (large transfer)
        pre_token_balances: vec![],
        post_token_balances: vec![],
        log_messages: vec![],
        compute_units_consumed: Some(20000),
    });

    println!("{}", style("Creating test transaction with large value transfer...").dim());
    
    match rule.evaluate(&test_event).await {
        Ok(Some(alert)) => {
            println!("{} Rule triggered alert:", style("✓").green().bold());
            println!("  Severity: {:?}", alert.severity);
            println!("  Message: {}", alert.message);
        }
        Ok(None) => {
            println!("{} Rule did not trigger", style("ⓘ").blue());
        }
        Err(e) => {
            println!("{} Rule evaluation failed: {}", style("✗").red().bold(), e);
        }
    }

    Ok(())
}

async fn test_oracle_deviation_rule() -> Result<()> {
    let rule = OracleDeviationRule::new(5.0, "reference_oracle".to_string());
    
    println!("{}", style("Oracle rule test requires live price data").dim());
    println!("{} Oracle deviation rule configured successfully", style("✓").green());
    println!("  Threshold: 5%");
    println!("  Reference: reference_oracle");

    Ok(())
}

async fn test_failure_rate_rule() -> Result<()> {
    use watchtower_engine::{SolanaEvent, TransactionEvent};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    let rule = FailureRateRule::new(25.0, 10, 300);
    
    println!("{}", style("Creating test transactions with high failure rate...").dim());
    
    // Simulate multiple failed transactions
    for i in 0..15 {
        let success = i < 5; // 5 successful, 10 failed = 66% failure rate
        
        let test_event = SolanaEvent::Transaction(TransactionEvent {
            signature: format!("test_signature_{}", i),
            slot: 12347 + i as u64,
            block_time: Some(chrono::Utc::now().timestamp()),
            program_id: Pubkey::from_str("11111111111111111111111111111112").unwrap(),
            accounts: vec![],
            instruction_data: vec![],
            success,
            error: if success { None } else { Some("Insufficient funds".to_string()) },
            fee: 5000,
            pre_balances: vec![1000000000],
            post_balances: vec![1000000000],
            pre_token_balances: vec![],
            post_token_balances: vec![],
            log_messages: vec![],
            compute_units_consumed: Some(5000),
        });

        if let Ok(Some(alert)) = rule.evaluate(&test_event).await {
            println!("{} Rule triggered alert after {} transactions:", style("✓").green().bold(), i + 1);
            println!("  Severity: {:?}", alert.severity);
            println!("  Message: {}", alert.message);
            return Ok(());
        }
    }
    
    println!("{} Rule did not trigger with test data", style("ⓘ").blue());
    Ok(())
} 