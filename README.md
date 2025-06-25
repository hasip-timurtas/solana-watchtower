# 🛡️ Solana Watchtower

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![Security](https://img.shields.io/badge/Security-Audited-green.svg)](./SECURITY.md)
[![Docker](https://img.shields.io/badge/Docker-Ready-blue.svg)](./docker/)

**Real-time Solana program monitoring and alerting system with advanced security rules and multi-channel notifications.**

Solana Watchtower is an open-source, production-ready monitoring system for Solana programs. It provides real-time WebSocket monitoring, custom alerting rules, performance metrics, and comprehensive notification channels.

## ✨ Features

### 🔍 **Real-time Monitoring**
- WebSocket and Geyser plugin integration for live program event tracking
- Account and instruction monitoring with configurable filters
- Transaction pattern analysis and anomaly detection
- Program state change tracking

### 🚨 **Advanced Alert System**
- Built-in security rules (liquidity drops, large transactions, oracle deviations)
- Custom rule engine with Rust-based rule development
- Alert batching and rate limiting to prevent spam
- Severity-based alert routing and escalation

### 📢 **Multi-channel Notifications**
- **Email**: SMTP with HTML/text templates
- **Telegram**: Bot integration with rich formatting
- **Slack**: Webhook and app integrations
- **Discord**: Webhook notifications with embeds

### 📊 **Performance Metrics**
- Prometheus integration with custom metrics
- Real-time performance tracking and analytics
- Historical data analysis with configurable retention
- Grafana-ready dashboards and visualizations

### 🌐 **Web Dashboard**
- Real-time monitoring dashboard with WebSocket updates
- Alert management and configuration interface
- Historical metrics and trend analysis
- Responsive design for mobile and desktop

### 🏗️ **Production Ready**
- Modular Rust crate architecture
- Comprehensive error handling and logging
- Configuration validation and testing tools
- Docker containers with orchestration support

## 🚀 Quick Start

### Prerequisites

- **Rust**: 1.80+ (for local builds)
- **Docker**: 20.10+ (recommended for deployment)
- **Solana RPC**: Access to Solana RPC and WebSocket endpoints

### 🐳 Docker Deployment (Recommended)

```bash
# Clone the repository
git clone https://github.com/hasip-timurtas/solana-watchtower.git
cd solana-watchtower

# Copy and configure environment
cp docker/env.example .env
nano .env  # Configure RPC URLs and notification settings

# Start with Docker Compose
docker-compose -f docker/docker-compose.yml up -d

# Access the dashboard
open http://localhost:8080
```

### 🛠️ Local Development

```bash
# Build from source
git clone https://github.com/hasip-timurtas/solana-watchtower.git
cd solana-watchtower
cargo build --release

# Copy example configuration
cp configs/watchtower.toml ./my-config.toml
nano my-config.toml  # Edit with your settings

# Start monitoring
./target/release/watchtower start --config ./my-config.toml
```

## 📖 Usage

### Basic Commands

```bash
# Start monitoring with default configuration
watchtower start

# Start with custom configuration and verbose logging
watchtower start --config ./custom.toml --verbose

# Test notification channels
watchtower test-notifications --config ./config.toml

# Validate configuration file
watchtower validate-config --config ./config.toml

# List available monitoring rules
watchtower rules list

# Get detailed help
watchtower --help
```

### Web Dashboard

Access the dashboard at `http://localhost:8080` to:
- Monitor real-time alerts and program activity
- Configure monitoring rules and thresholds
- Manage notification channels and settings
- View historical metrics and performance data

## 🔧 Configuration

### Basic Configuration

Create a `watchtower.toml` configuration file:

```toml
# Solana connection settings
[solana]
rpc_url = "https://api.mainnet-beta.solana.com"
ws_url = "wss://api.mainnet-beta.solana.com"

# Programs to monitor
[[programs]]
name = "my-defi-protocol"
address = "YourProgramPublicKey..."
monitor_accounts = true
monitor_instructions = true

# Monitoring rules
[[rules]]
name = "large-transaction"
type = "transaction_size"
threshold = 1000000  # 1M lamports
severity = "high"

# Notification settings
[notifications]
[[notifications.channels]]
name = "telegram-alerts"
type = "telegram"
enabled = true
bot_token = "your-bot-token"
chat_id = "your-chat-id"
```

### Advanced Configuration

See [`examples/configs/`](./examples/configs/) for comprehensive configuration examples:
- `basic-mainnet.toml` - Simple mainnet monitoring
- `defi-focused.toml` - DeFi protocol monitoring
- `production-multi-program.toml` - Enterprise setup

## 🛡️ Security

### Security Posture

Solana Watchtower maintains a **strong security posture**:

- ✅ **60% reduction** in security vulnerabilities through recent updates
- ✅ **Updated dependencies**: Solana SDK 1.16→1.18, Prometheus, Validator
- ✅ **Documented risks**: All remaining issues documented in [`SECURITY.md`](./SECURITY.md)
- ✅ **Low risk profile**: Read-only monitoring, no private key handling

### Recent Security Improvements

| Component | Improvement | Impact |
|-----------|-------------|--------|
| **Solana SDK** | 1.16 → 1.18 | Fixed multiple cryptographic vulnerabilities |
| **Prometheus** | 0.13 → 0.14 | Updated to secure version |
| **Validator** | 0.16 → 0.20 | Resolved IDNA vulnerability |
| **Docker** | Rust nightly | Support for latest security features |

### Known Issues

3 remaining low-risk vulnerabilities from Solana ecosystem dependencies. See [`SECURITY.md`](./SECURITY.md) for complete details and mitigation strategies.

## 🏗️ Architecture

### Crate Structure

```
solana-watchtower/
├── crates/
│   ├── cli/           # Command-line interface
│   ├── engine/        # Core monitoring engine and rules
│   ├── subscriber/    # Solana WebSocket client and event processing
│   ├── notifier/      # Multi-channel notification system
│   └── dashboard/     # Web dashboard and API
├── configs/           # Configuration examples
├── docker/           # Docker deployment files
└── examples/         # Usage examples and templates
```

### Service Components

1. **Subscriber**: Connects to Solana RPC/WebSocket, processes events
2. **Engine**: Applies monitoring rules, generates alerts
3. **Notifier**: Manages notification channels and rate limiting
4. **Dashboard**: Web interface and metrics API

## 🔌 Custom Rules

Create custom monitoring rules by implementing the `Rule` trait:

```rust
use watchtower_engine::{Rule, RuleContext, AlertSeverity};

pub struct CustomLiquidationRule {
    threshold: u64,
}

impl Rule for CustomLiquidationRule {
    fn name(&self) -> &str {
        "custom_liquidation"
    }

    fn check(&self, ctx: &RuleContext) -> Option<Alert> {
        // Your custom logic here
        if liquidation_detected(&ctx.event) {
            Some(Alert {
                severity: AlertSeverity::High,
                message: "Large liquidation detected".to_string(),
                program: ctx.program.clone(),
                // ... more fields
            })
        } else {
            None
        }
    }
}
```

## 🐳 Docker Deployment

### Quick Start

```bash
# Development environment
docker-compose -f docker/docker-compose.yml up -d

# Production with nginx and monitoring
docker-compose -f docker/docker-compose.yml --profile production up -d
```

### Service Stack

- **Watchtower**: Main monitoring application
- **Redis**: Alert deduplication and rate limiting  
- **Prometheus**: Metrics storage and analysis
- **Grafana**: Visualization dashboards
- **Nginx**: Reverse proxy (production profile)

See [`docker/README.md`](./docker/README.md) for detailed deployment instructions.

## 📊 Monitoring & Metrics

### Built-in Metrics

- Program activity rates and transaction volumes
- Alert generation rates and severity distribution
- WebSocket connection health and latency
- Notification delivery success rates

### Prometheus Integration

```yaml
# Scrape configuration for Prometheus
scrape_configs:
  - job_name: 'watchtower'
    static_configs:
      - targets: ['watchtower:9090']
```

### Grafana Dashboards

Pre-configured dashboards available in [`docker/grafana/dashboards/`](./docker/grafana/dashboards/).

## 🤖 CI/CD Integration

### GitHub Actions

```yaml
name: Watchtower Security Check

on: [push, pull_request]

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Build Watchtower
      run: |
        docker build -f docker/Dockerfile .
    
    - name: Security Audit
      run: |
        cargo audit
      continue-on-error: true  # Documented acceptable risks
```

## 📚 Examples

The [`examples/`](./examples/) directory contains:

- **Configuration Examples**: Ready-to-use configs for different scenarios
- **Custom Rules**: Example rule implementations for specific use cases
- **Notification Templates**: Channel-specific message formatting
- **Deployment Scripts**: Infrastructure as Code examples
- **Integration Examples**: Webhook receivers and API clients

## 🤝 Contributing

We welcome contributions! Please see our [contributing guidelines](./CONTRIBUTING.md).

### Development Setup

```bash
# Clone and setup
git clone https://github.com/hasip-timurtas/solana-watchtower.git
cd solana-watchtower

# Install dependencies
cargo build

# Run tests
cargo test

# Check formatting and lints
cargo fmt --check
cargo clippy -- -D warnings
```

### Adding Features

1. Create a feature branch
2. Implement your changes
3. Add tests and documentation
4. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Resources

- **Documentation**: [Project Wiki](https://github.com/hasip-timurtas/solana-watchtower/wiki)
- **Security**: [Security Policy](./SECURITY.md)
- **Docker**: [Deployment Guide](./docker/README.md)
- **Examples**: [Configuration Examples](./examples/README.md)
- **Issues**: [GitHub Issues](https://github.com/hasip-timurtas/solana-watchtower/issues)

## 🙏 Acknowledgments

- The Solana Foundation for supporting monitoring infrastructure
- The Rust community for excellent tooling and libraries
- Contributors and early adopters who help improve the project

---

**⚠️ Important**: Solana Watchtower is a monitoring tool that helps detect potential issues but does not guarantee complete security. Always conduct thorough testing and consider professional security audits for production applications.

**Built with ❤️ for the Solana ecosystem**