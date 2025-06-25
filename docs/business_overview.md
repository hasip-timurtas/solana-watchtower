# Solana Watchtower: Business & Project Overview

## Overview
Solana Watchtower is an open-source monitoring and alerting solution for programs deployed on the Solana blockchain. It provides real-time WebSocket monitoring, custom security rules, performance metrics, and notifications over email or chat services.

## Key Features
- Real-time Monitoring using WebSocket and Geyser plugin integration
- Custom Rule Engine with built-in security rules and support for custom rules
- Multi-channel Alerts via Email (SMTP), Telegram, Slack, and Discord
- Performance Metrics with Prometheus integration
- Intelligent Rate Limiting and alert batching
- Web Dashboard for real-time monitoring and alert management
- Modular architecture with extensible crates
- Production-ready error handling and configuration validation

## Quick Start
1. Clone the repository and build:
   ```bash
   git clone https://github.com/hasip-timurtas/solana-watchtower.git
   cd solana-watchtower
   cargo build --release
   ```
2. Copy and edit configuration:
   ```bash
   cp configs/watchtower.toml ./my-watchtower.toml
   nano my-watchtower.toml
   ```
3. Start monitoring:
   ```bash
   watchtower start --config ./my-watchtower.toml
   ```

## Command Line Highlights
- `watchtower start`: run monitoring with optional dashboard and metrics ports
- `watchtower test-notifications`: verify email and chat integrations
- `watchtower validate-config`: check configuration syntax
- `watchtower rules`: list, inspect, or test built-in rules

## Docker Deployment
The `docker/` folder provides a compose setup for production deployment.
Steps include copying `docker/env.example` to `.env`, configuring RPC URLs and notification credentials, then running `docker-compose -f docker/docker-compose.yml up -d`.

## Architecture
Watchtower consists of several Rust crates:
- **subscriber**: receives events from Solana WebSockets
- **engine**: evaluates events against rule sets
- **notifier**: sends alerts to configured channels
- **dashboard**: serves a web interface and exposes Prometheus metrics
- **cli**: command-line entry point for starting and managing the system

## Business Perspective
Solana Watchtower helps teams operating on Solana maintain visibility and react quickly to potential issues such as security threats, large transactions, or liquidity events. By integrating with common notification channels and providing a user-friendly dashboard, it aims to reduce operational risk and downtime. The tool is released under the MIT license, encouraging community contributions and integration into broader security strategies.
