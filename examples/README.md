# Solana Watchtower Examples

This directory contains comprehensive examples for configuring and using Solana Watchtower to monitor various Solana programs and scenarios.

## Quick Start

1. **Basic Configuration**: Start with `configs/basic-mainnet.toml` for a simple setup
2. **Development**: Use `configs/devnet-testing.toml` for testing on devnet
3. **Production**: See `configs/production-multi-program.toml` for advanced setups

## Directory Structure

### 📁 `configs/`
Complete configuration examples for different environments and use cases:
- `basic-mainnet.toml` - Simple mainnet monitoring setup
- `devnet-testing.toml` - Development and testing configuration
- `production-multi-program.toml` - Production setup with multiple programs
- `defi-focused.toml` - DeFi protocol monitoring
- `nft-marketplace.toml` - NFT marketplace monitoring

### 📁 `rules/`
Custom rule implementations for specific monitoring scenarios:
- `defi-liquidation-rules.rs` - Monitor DeFi liquidation events
- `nft-volume-rules.rs` - Track NFT trading volume and anomalies
- `token-supply-rules.rs` - Monitor token supply changes
- `whale-activity-rules.rs` - Detect large holder movements

### 📁 `notifications/`
Template examples for different notification channels:
- `discord/` - Discord webhook templates
- `slack/` - Slack notification formats
- `telegram/` - Telegram bot messages
- `email/` - HTML and text email templates

### 📁 `integrations/`
Integration examples with external systems:
- `webhook-receiver.py` - Python webhook receiver example
- `alertmanager-config.yml` - Prometheus AlertManager integration
- `datadog-forwarder.js` - DataDog metrics forwarding
- `custom-api-client.rs` - Custom API client for alerts

### 📁 `deployment/`
Deployment scenarios and infrastructure examples:
- `kubernetes/` - Kubernetes deployment manifests
- `terraform/` - Infrastructure as Code examples
- `systemd/` - Linux systemd service configuration
- `monitoring-stack/` - Complete monitoring infrastructure

### 📁 `programs/`
Real Solana program examples and their monitoring configurations:
- `spl-token/` - SPL Token program monitoring
- `serum-dex/` - Serum DEX monitoring setup
- `lending-protocols/` - Lending protocol examples
- `nft-marketplaces/` - NFT marketplace monitoring

## Usage Examples

### Monitor a DeFi Protocol

```bash
# Copy the DeFi configuration
cp examples/configs/defi-focused.toml configs/watchtower.toml

# Edit with your specific programs
nano configs/watchtower.toml

# Start monitoring
watchtower-cli start
```

### Set Up Development Environment

```bash
# Use devnet configuration
cp examples/configs/devnet-testing.toml configs/watchtower.toml

# Test notifications
watchtower-cli test-notifications
```

### Deploy Production Setup

```bash
# Use production configuration
cp examples/configs/production-multi-program.toml configs/watchtower.toml

# Deploy with Docker
docker-compose -f examples/deployment/docker/docker-compose.prod.yml up -d
```

## Configuration Patterns

### Basic Program Monitoring
```toml
[[programs]]
name = "my-program"
address = "YourProgramAddress..."
monitor_accounts = true
monitor_instructions = true
```

### Advanced Rule Configuration
```toml
[[rules]]
name = "large-transaction"
type = "transaction_size"
threshold = 1000000  # 1M lamports
window = "5m"
severity = "high"
```

### Multi-Channel Notifications
```toml
[[notifications.channels]]
name = "critical-alerts"
type = "telegram"
enabled = true
filter = { severity = ["critical", "high"] }
```

## Testing Examples

### Validate Configuration
```bash
watchtower-cli validate-config --config examples/configs/basic-mainnet.toml
```

### Test Rules
```bash
watchtower-cli test-rules --config examples/configs/defi-focused.toml
```

### Simulate Alerts
```bash
watchtower-cli simulate-alert --type liquidity_drop --severity high
```

## Best Practices

1. **Start Simple**: Begin with `basic-mainnet.toml` and gradually add complexity
2. **Test First**: Always test configurations on devnet before production
3. **Monitor Gradually**: Add programs one at a time to avoid alert fatigue
4. **Customize Rules**: Adapt rule thresholds based on your program's behavior
5. **Backup Configs**: Version control your configuration files

## Contributing Examples

To contribute new examples:

1. Create appropriate directory structure
2. Include comprehensive documentation
3. Test thoroughly on devnet
4. Add to this README
5. Submit PR with example description

## Support

For questions about examples:
- Check the main project documentation
- Review similar configurations in this directory
- Open an issue with the `examples` label 