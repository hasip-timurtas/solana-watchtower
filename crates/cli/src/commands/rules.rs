use anyhow::Result;
use console::style;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use watchtower_engine::{
    FailureRateRule, LargeTransactionRule, LiquidityDropRule, OracleDeviationRule, Rule,
    RuleContext,
};
use watchtower_subscriber::{EventData, EventType, ProgramEvent};

pub async fn rules_list_command() -> Result<()> {
    println!("{}", style("Available Monitoring Rules:").bold());
    println!("{}", "─".repeat(60));

    let rules = [
        (
            "liquidity_drop",
            "Liquidity Drop Detection",
            "Monitors for sudden drops in liquidity pools",
        ),
        (
            "large_transaction",
            "Large Transaction Detection",
            "Flags unusually large transactions",
        ),
        (
            "oracle_deviation",
            "Oracle Price Deviation",
            "Detects price manipulation attempts",
        ),
        (
            "failure_rate",
            "High Failure Rate Detection",
            "Monitors transaction failure rates",
        ),
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

    println!(
        "{}",
        style("Use 'watchtower rules info <rule_name>' for detailed information").dim()
    );
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
    let rule = LiquidityDropRule::new(10.0, 300, 1000000);

    // Create test event with token transfer data
    let test_event = ProgramEvent::new(
        Pubkey::from_str("11111111111111111111111111111112").unwrap(),
        "Test Program".to_string(),
        EventType::TokenTransfer,
        EventData::TokenTransfer {
            from: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 100000, // Large amount to trigger liquidity drop
            mint: Pubkey::new_unique(),
            decimals: 6,
        },
    )
    .with_slot(12345);

    // Create test context with historical data
    let context = RuleContext::default();

    println!(
        "{}",
        style("Creating test token transfer with potential liquidity impact...").dim()
    );

    let result = rule.evaluate(&test_event, &context).await;

    if result.triggered {
        println!("{} Rule triggered alert:", style("✓").green().bold());
        println!("  Severity: {:?}", result.severity);
        if let Some(message) = &result.message {
            println!("  Message: {}", message);
        }
        println!("  Confidence: {:.2}", result.confidence);
        println!("  Metadata: {:?}", result.metadata);
    } else {
        println!(
            "{} Rule did not trigger (no significant liquidity drop detected)",
            style("ⓘ").blue()
        );
    }

    Ok(())
}

async fn test_large_transaction_rule() -> Result<()> {
    let rule = LargeTransactionRule::new(1.0, 500000);

    // Create test event with large token transfer
    let test_event = ProgramEvent::new(
        Pubkey::from_str("11111111111111111111111111111112").unwrap(),
        "Test Program".to_string(),
        EventType::TokenTransfer,
        EventData::TokenTransfer {
            from: Pubkey::new_unique(),
            to: Pubkey::new_unique(),
            amount: 1_000_000, // Large amount above threshold
            mint: Pubkey::new_unique(),
            decimals: 6,
        },
    )
    .with_slot(12346);

    // Create test context
    let context = RuleContext::default();

    println!(
        "{}",
        style("Creating test transaction with large value transfer...").dim()
    );

    let result = rule.evaluate(&test_event, &context).await;

    if result.triggered {
        println!("{} Rule triggered alert:", style("✓").green().bold());
        println!("  Severity: {:?}", result.severity);
        if let Some(message) = &result.message {
            println!("  Message: {}", message);
        }
        println!("  Confidence: {:.2}", result.confidence);
    } else {
        println!("{} Rule did not trigger", style("ⓘ").blue());
    }

    Ok(())
}

async fn test_oracle_deviation_rule() -> Result<()> {
    let _rule = OracleDeviationRule::new(5.0, "reference_oracle".to_string());

    println!(
        "{}",
        style("Oracle rule test requires live price data").dim()
    );
    println!(
        "{} Oracle deviation rule configured successfully",
        style("✓").green()
    );
    println!("  Threshold: 5%");
    println!("  Reference: reference_oracle");

    Ok(())
}

async fn test_failure_rate_rule() -> Result<()> {
    let rule = FailureRateRule::new(25.0, 10, 300);

    println!(
        "{}",
        style("Creating test transactions with high failure rate...").dim()
    );

    // Create a context with multiple failed transactions
    let mut context = RuleContext::default();

    // Add historical failed transactions to context
    for i in 0..15 {
        let success = i < 5; // 5 successful, 10 failed = 66% failure rate

        let test_event = ProgramEvent::new(
            Pubkey::from_str("11111111111111111111111111111112").unwrap(),
            "Test Program".to_string(),
            EventType::Transaction,
            EventData::Transaction {
                signature: solana_sdk::signature::Signature::new_unique(),
                success,
                compute_units: Some(5000),
                fee: 5000,
            },
        )
        .with_slot(12347 + i as u64);

        context.recent_events.push(test_event);
    }

    // Create a current transaction event to evaluate
    let current_event = ProgramEvent::new(
        Pubkey::from_str("11111111111111111111111111111112").unwrap(),
        "Test Program".to_string(),
        EventType::Transaction,
        EventData::Transaction {
            signature: solana_sdk::signature::Signature::new_unique(),
            success: false, // This is a failed transaction
            compute_units: Some(5000),
            fee: 5000,
        },
    )
    .with_slot(12362);

    let result = rule.evaluate(&current_event, &context).await;

    if result.triggered {
        println!("{} Rule triggered alert:", style("✓").green().bold());
        println!("  Severity: {:?}", result.severity);
        if let Some(message) = &result.message {
            println!("  Message: {}", message);
        }
        println!("  Confidence: {:.2}", result.confidence);
        println!("  Metadata: {:?}", result.metadata);
    } else {
        println!("{} Rule did not trigger with test data", style("ⓘ").blue());
    }

    Ok(())
}
