#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1"
}

# Default values
DEFAULT_CONFIG_PATH="/app/configs/watchtower.toml"
DEFAULT_SERVICE="dashboard"

# Parse command line arguments
SERVICE=${1:-$DEFAULT_SERVICE}
CONFIG_PATH=${CONFIG_PATH:-$DEFAULT_CONFIG_PATH}

log "Starting Solana Watchtower..."
log "Service: $SERVICE"
log "Config path: $CONFIG_PATH"

# Validate config file exists
if [ ! -f "$CONFIG_PATH" ]; then
    error "Configuration file not found at $CONFIG_PATH"
    exit 1
fi

# Create necessary directories
mkdir -p /app/data /app/logs

# Validate configuration
log "Validating configuration..."
if ! watchtower-cli validate-config --config "$CONFIG_PATH"; then
    error "Configuration validation failed"
    exit 1
fi

# Function to start the dashboard service
start_dashboard() {
    log "Starting Watchtower Dashboard..."
    exec watchtower-dashboard --config "$CONFIG_PATH" --host 0.0.0.0 --port 8080
}

# Function to start the monitoring engine
start_engine() {
    log "Starting Watchtower Engine..."
    exec watchtower-cli start --config "$CONFIG_PATH" --daemon
}

# Function to start both services
start_all() {
    log "Starting all Watchtower services..."
    
    # Start engine in background
    watchtower-cli start --config "$CONFIG_PATH" --daemon &
    ENGINE_PID=$!
    
    # Start dashboard in foreground
    watchtower-dashboard --config "$CONFIG_PATH" --host 0.0.0.0 --port 8080 &
    DASHBOARD_PID=$!
    
    # Wait for either process to exit
    wait $ENGINE_PID $DASHBOARD_PID
}

# Handle different service types
case "$SERVICE" in
    "dashboard")
        start_dashboard
        ;;
    "engine")
        start_engine
        ;;
    "all")
        start_all
        ;;
    "validate")
        log "Validating configuration only..."
        watchtower-cli validate-config --config "$CONFIG_PATH"
        ;;
    "test-notifications")
        log "Testing notification channels..."
        watchtower-cli test-notifications --config "$CONFIG_PATH"
        ;;
    *)
        error "Unknown service: $SERVICE"
        echo "Available services: dashboard, engine, all, validate, test-notifications"
        exit 1
        ;;
esac 