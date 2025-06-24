# 🛡️ Solana Watchtower

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)

An open-source, end-to-end monitoring and alert system for deployed Solana programs. Real-time WebSocket monitoring with custom risk rules, performance metrics, and multi-channel notifications.

## ✨ Features

- **Real-time Monitoring**: WebSocket and Geyser plugin integration for live program event tracking
- **Custom Rule Engine**: Built-in security rules with support for custom rule development
- **Multi-channel Alerts**: Email (SMTP), Telegram, Slack, and Discord notifications
- **Performance Metrics**: Prometheus integration with sliding window analytics
- **Rate Limiting**: Intelligent rate limiting and alert batching to prevent spam
- **Web Dashboard**: Real-time monitoring dashboard with alert management
- **Modular Architecture**: Extensible crate-based design with clean APIs
- **Production Ready**: Comprehensive error handling, logging, and configuration validation

## 🚀 Quick Start

### Installation

#### From Source
```bash
git clone https://github.com/hasip-timurtas/solana-watchtower.git
cd solana-watchtower
cargo build --release
```

#### Binary Installation
```bash
# The binary will be available at target/release/watchtower
cp target/release/watchtower /usr/local/bin/
```

### Configuration

Create a configuration file based on the example:

```bash
# Copy example configuration
cp configs/watchtower.toml ./my-watchtower.toml

# Edit configuration with your settings
nano my-watchtower.toml
```

### Basic Usage

```bash
# Start monitoring with default configuration
watchtower start

# Start with custom configuration
watchtower start --config ./my-watchtower.toml

# Test notification channels
watchtower test-notifications --config ./my-watchtower.toml

# Validate configuration
watchtower validate-config --config ./my-watchtower.toml

# View help
watchtower --help
```

## 📖 Commands

### `watchtower start`

Start the monitoring system with real-time WebSocket connections to Solana.

```bash
watchtower start [OPTIONS]

OPTIONS:
    -c, --config <FILE>          Configuration file path [default: ./watchtower.toml]
    -v, --verbose                Enable verbose logging
    -d, --daemon                 Run as background daemon
        --dashboard-port <PORT>  Dashboard port [default: 8080]
        --metrics-port <PORT>    Prometheus metrics port [default: 9090]

EXAMPLES:
    # Start with default configuration
    watchtower start

    # Start with custom config and verbose logging
    watchtower start --config ./my-config.toml --verbose
    
    # Run as daemon with custom ports
    watchtower start --daemon --dashboard-port 3000 --metrics-port 3001
```

### `watchtower test-notifications`

Test all configured notification channels.

```bash
watchtower test-notifications [OPTIONS]

OPTIONS:
    -c, --config <FILE>          Configuration file path
    -t, --channel <CHANNEL>      Test specific channel (email, telegram, slack, discord)

EXAMPLES:
    # Test all channels
    watchtower test-notifications
    
    # Test only email
    watchtower test-notifications --channel email
```

### `watchtower validate-config`

Validate configuration file syntax and settings.

```bash
watchtower validate-config [OPTIONS]

OPTIONS:
    -c, --config <FILE>          Configuration file path

EXAMPLES:
    watchtower validate-config --config ./watchtower.toml
```

### `watchtower rules`

Manage monitoring rules.

```bash
watchtower rules <ACTION> [OPTIONS]

ACTIONS:
    list                List available rules
    info <RULE_NAME>    Show rule information
    test <RULE_NAME>    Test rule with sample data

EXAMPLES:
    watchtower rules list
    watchtower rules info liquidity_drop
    watchtower rules test large_transaction
```

## 🔧 Configuration

Create a `solsec.toml` configuration file:

```toml
# Enable/disable specific rules
enabled_rules = [
    "integer_overflow",
    "missing_signer_check", 
    "unchecked_account",
    "reentrancy"
]

disabled_rules = []

# Rule-specific settings
[rule_settings]
[rule_settings.integer_overflow]
ignore_patterns = ["test_*", "mock_*"]

[rule_settings.missing_signer_check]
required_for_instructions = ["transfer", "withdraw"]
```

## 🔍 Built-in Security Rules

| Rule | Severity | Description |
|------|----------|-------------|
| `integer_overflow` | Medium | Detects potential integer overflow vulnerabilities |
| `missing_signer_check` | High | Identifies missing signer validation in instruction handlers |
| `unchecked_account` | Critical | Finds accounts used without proper validation |
| `reentrancy` | High | Detects potential reentrancy vulnerabilities |

## 🔌 Plugin Development

Create custom security rules by implementing the `Rule` trait:

```rust
use solsec::plugin::{Rule, RuleResult, Severity};
use std::path::Path;
use anyhow::Result;

#[derive(Debug)]
pub struct MyCustomRule;

impl Rule for MyCustomRule {
    fn name(&self) -> &str {
        "my_custom_rule"
    }

    fn description(&self) -> &str {
        "Detects my specific vulnerability pattern"
    }

    fn check(&self, content: &str, file_path: &Path) -> Result<Vec<RuleResult>> {
        let mut results = Vec::new();
        
        // Your analysis logic here
        for (line_num, line) in content.lines().enumerate() {
            if line.contains("dangerous_pattern") {
                results.push(RuleResult {
                    severity: Severity::High,
                    message: "Dangerous pattern detected".to_string(),
                    line_number: Some(line_num + 1),
                    column: None,
                    code_snippet: Some(line.trim().to_string()),
                    suggestion: Some("Use safe alternative".to_string()),
                });
            }
        }
        
        Ok(results)
    }
}

// Plugin interface
#[no_mangle]
pub extern "C" fn get_plugin_info() -> PluginInfo {
    PluginInfo {
        name: "my_plugin".to_string(),
        version: "1.0.0".to_string(),
        description: "My custom security plugin".to_string(),
        author: "Your Name".to_string(),
        rules: vec!["my_custom_rule".to_string()],
    }
}

#[no_mangle]
pub extern "C" fn create_rules() -> Vec<Box<dyn Rule>> {
    vec![Box::new(MyCustomRule)]
}
```

Build your plugin as a dynamic library:

```bash
cargo build --lib --crate-type=cdylib --release
```

## 🤖 CI/CD Integration

### GitHub Actions

Add the following to your `.github/workflows/security.yml`:

```yaml
name: Security Scan

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  security:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    
    - name: Install solsec
      run: |
        curl -L https://github.com/hasip-timurtas/solsec/releases/latest/download/solsec-linux-x86_64.tar.gz | tar xz
        sudo mv solsec /usr/local/bin/
    
    - name: Run security scan
      run: |
        solsec scan ./programs --output ./security-results
    
    - name: Upload security report
      uses: actions/upload-artifact@v3
      with:
        name: security-report
        path: ./security-results/
    
    - name: Fail on critical issues
      run: |
        if [ -f ./security-results/*.json ]; then
          critical_count=$(jq '[.[] | select(.severity == "critical")] | length' ./security-results/*.json)
          if [ "$critical_count" -gt 0 ]; then
            echo "❌ Critical security issues found!"
            exit 1
          fi
        fi
```

### Pre-commit Hook

```bash
#!/bin/sh
# .git/hooks/pre-commit

echo "🛡️ Running security scan..."
solsec scan ./programs --format json --output ./tmp-security-results

if [ -f ./tmp-security-results/*.json ]; then
    critical_count=$(jq '[.[] | select(.severity == "critical")] | length' ./tmp-security-results/*.json 2>/dev/null || echo "0")
    if [ "$critical_count" -gt 0 ]; then
        echo "❌ Critical security issues found! Commit blocked."
        echo "Run 'solsec scan ./programs' to see details."
        rm -rf ./tmp-security-results
        exit 1
    fi
fi

rm -rf ./tmp-security-results
echo "✅ Security scan passed!"
```

## Browser Opening Behavior

HTML reports automatically open in the default browser under the following conditions:

**Opens automatically when:**
- Running in an interactive terminal (not redirected)
- Generating HTML reports (`--html-only` or default formats)
- Not in CI/automation environments

**Remains closed when:**
- Running in CI environments (GitHub Actions, GitLab CI, etc.)
- Output is redirected or piped
- Using `--no-open` flag
- Only generating non-visual formats (JSON, CSV)

## 📊 Report Examples

### HTML Report
Interactive HTML reports with:
- Executive summary with issue counts by severity
- Detailed findings with code snippets
- Actionable recommendations
- Responsive design for all devices

### JSON Report
Machine-readable format for:
- CI/CD pipeline integration
- Custom tooling and analysis
- Data processing and metrics

### Markdown Report
Developer-friendly format for:
- README documentation
- Pull request comments
- Documentation sites

## 🛠️ Development

### Building from Source

```bash
git clone https://github.com/hasip-timurtas/solsec.git
cd solsec
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📚 Examples

The [`examples/`](./examples/) directory contains comprehensive security vulnerability demonstrations:

### 🚨 Vulnerability Examples
Each category includes both **vulnerable** and **secure** implementations for educational purposes:

| Vulnerability Type | Severity | Vulnerable Examples | Secure Examples |
|-------------------|----------|-------------------|-----------------|
| **Integer Overflow** | Medium | `examples/integer_overflow/vulnerable.rs` | `examples/integer_overflow/secure.rs` |
| **Missing Signer Check** | High | `examples/missing_signer_check/vulnerable.rs` | `examples/missing_signer_check/secure.rs` |
| **Unchecked Account** | Critical | `examples/unchecked_account/vulnerable.rs` | `examples/unchecked_account/secure.rs` |
| **Reentrancy** | High | `examples/reentrancy/vulnerable.rs` | `examples/reentrancy/secure.rs` |

### 🧪 Testing the Examples

```bash
# Test vulnerable examples (should find many issues)
solsec scan examples/integer_overflow/vulnerable.rs     # 5 issues found
solsec scan examples/missing_signer_check/vulnerable.rs # 5 issues found
solsec scan examples/unchecked_account/vulnerable.rs    # 6 issues found
solsec scan examples/reentrancy/vulnerable.rs           # 2 issues found

# Test secure examples (should find 0 issues)
solsec scan examples/*/secure.rs                        # No issues found

# Comprehensive analysis
solsec scan examples/                                    # 26 total issues across all vulnerable examples
```

### 📖 Learning Resources
- **Side-by-side Comparisons**: See exactly how to fix each vulnerability
- **Real-world Patterns**: Actual Solana/Anchor code patterns
- **Educational Comments**: Clear explanations of security issues
- **Test Suite**: Validate that solsec detection works correctly

See the detailed [`examples/README.md`](./examples/README.md) for complete documentation.

## 🤝 Community

- **Issues**: [GitHub Issues](https://github.com/hasip-timurtas/solsec/issues)
- **Discussions**: [GitHub Discussions](https://github.com/hasip-timurtas/solsec/discussions)
- **Discord**: [Solana Security Community](https://discord.gg/solana-security)

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- The Solana Foundation for supporting security tooling
- The Rust security community for best practices
- Contributors and early adopters

---

**⚠️ Important**: This tool helps identify potential security issues but does not guarantee complete security. Always conduct thorough testing and consider professional security audits for production applications.

*Built with ❤️ by Hasip Timurtas*