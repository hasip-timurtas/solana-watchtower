# Telegram Liquidation Alert Template

## Template: liquidation_alert

```markdown
🔴 *LIQUIDATION ALERT*

*Protocol:* {{alert.program_name}}
*Severity:* {{alert.severity | upper}}
*Amount:* {{alert.metadata.liquidation_amount_sol}} SOL

*Details:*
• Health Factor: {{alert.metadata.health_factor}}
• Liquidated Asset: {{alert.metadata.asset_name}}
• Liquidator: {{alert.metadata.liquidator | truncate(8)}}...

*Transaction:* [{{alert.signature | truncate(8)}}...](https://solscan.io/tx/{{alert.signature}})

*Time:* {{alert.timestamp}}

_Solana Watchtower Monitoring_
```

## Usage

This template is used for DeFi liquidation events and provides:
- Clear severity indication with emojis
- Key liquidation metrics
- Direct link to transaction
- Formatted for mobile viewing

## Variables Available

- `alert.program_name` - Protocol name (e.g., "Solend", "Mango")
- `alert.severity` - Alert severity level
- `alert.metadata.liquidation_amount_sol` - Amount liquidated in SOL
- `alert.metadata.health_factor` - Account health factor before liquidation
- `alert.metadata.asset_name` - Name of liquidated asset
- `alert.metadata.liquidator` - Liquidator's public key
- `alert.signature` - Transaction signature
- `alert.timestamp` - Alert timestamp 