#!/bin/bash
set -e

# Solana Watchtower Docker Setup Script
# This script helps set up and deploy Solana Watchtower using Docker

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

info() {
    echo -e "${BLUE}[SETUP]${NC} $1"
}

# Default values
ENVIRONMENT="production"
CONFIG_FILE=""
SKIP_BUILD=false
DETACHED=true

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --dev|--development)
            ENVIRONMENT="development"
            shift
            ;;
        --prod|--production)
            ENVIRONMENT="production"
            shift
            ;;
        --config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        --no-detach)
            DETACHED=false
            shift
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --help|-h)
            cat << EOF
Solana Watchtower Docker Setup Script

Usage: $0 [OPTIONS]

Options:
    --dev, --development    Deploy in development mode
    --prod, --production    Deploy in production mode (default)
    --config FILE          Use specific configuration file
    --no-detach            Run in foreground (don't use -d flag)
    --skip-build           Skip building images
    --help, -h             Show this help message

Examples:
    $0                                    # Production deployment
    $0 --dev                              # Development deployment
    $0 --config ./my-config.toml          # Custom config
    $0 --dev --no-detach                  # Development with logs

EOF
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Print banner
cat << 'EOF'
  ____        _                   __        __    _       _     _                          
 / ___|  ___ | | __ _ _ __   __ _  \ \      / /_ _| |_ ___| |__ | |_ _____      _____ _ __ 
 \___ \ / _ \| |/ _` | '_ \ / _` |  \ \ /\ / / _` | __/ __| '_ \| __/ _ \ \ /\ / / _ \ '__|
  ___) | (_) | | (_| | | | | (_| |   \ V  V / (_| | || (__| | | | || (_) \ V  V /  __/ |   
 |____/ \___/|_|\__,_|_| |_|\__,_|    \_/\_/ \__,_|\__\___|_| |_|\__\___/ \_/\_/ \___|_|   
                                                                                          
EOF

log "Starting Solana Watchtower Docker setup..."
info "Environment: $ENVIRONMENT"

# Check prerequisites
info "Checking prerequisites..."

if ! command -v docker &> /dev/null; then
    error "Docker is not installed. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    error "Docker Compose is not installed. Please install Docker Compose first."
    exit 1
fi

# Check Docker daemon
if ! docker info &> /dev/null; then
    error "Docker daemon is not running. Please start Docker first."
    exit 1
fi

log "Prerequisites check passed!"

# Set up environment file
if [ ! -f ".env" ]; then
    info "Creating environment file..."
    if [ -f "docker/env.example" ]; then
        cp docker/env.example .env
        log "Environment file created from template"
        warn "Please edit .env file with your configuration before continuing"
        read -p "Press Enter to continue after editing .env file..."
    else
        error "Template environment file not found at docker/env.example"
        exit 1
    fi
else
    log "Environment file already exists"
fi

# Set up configuration
if [ -n "$CONFIG_FILE" ]; then
    if [ -f "$CONFIG_FILE" ]; then
        info "Using custom configuration file: $CONFIG_FILE"
    else
        error "Configuration file not found: $CONFIG_FILE"
        exit 1
    fi
elif [ ! -f "configs/watchtower.toml" ]; then
    warn "No configuration file found at configs/watchtower.toml"
    warn "Make sure to create a proper configuration file"
fi

# Build images if not skipping
if [ "$SKIP_BUILD" = false ]; then
    info "Building Docker images..."
    if [ "$ENVIRONMENT" = "development" ]; then
        docker-compose -f docker/docker-compose.dev.yml build
    else
        docker-compose -f docker/docker-compose.yml build
    fi
    log "Images built successfully!"
fi

# Prepare docker-compose command
COMPOSE_FILE="docker/docker-compose.yml"
COMPOSE_CMD="docker-compose -f $COMPOSE_FILE"

if [ "$ENVIRONMENT" = "development" ]; then
    COMPOSE_FILE="docker/docker-compose.dev.yml"
    COMPOSE_CMD="docker-compose -f $COMPOSE_FILE"
fi

# Add detached flag if needed
if [ "$DETACHED" = true ]; then
    COMPOSE_CMD="$COMPOSE_CMD up -d"
else
    COMPOSE_CMD="$COMPOSE_CMD up"
fi

# Start services
info "Starting services..."
log "Command: $COMPOSE_CMD"

if eval $COMPOSE_CMD; then
    log "Services started successfully!"
    
    if [ "$DETACHED" = true ]; then
        echo ""
        info "Services are running in the background"
        info "Access points:"
        echo "  - Watchtower Dashboard: http://localhost:8080"
        
        if [ "$ENVIRONMENT" = "production" ]; then
            echo "  - Grafana: http://localhost:3000 (admin/admin)"
            echo "  - Prometheus: http://localhost:9091"
        fi
        
        echo ""
        info "Useful commands:"
        echo "  - View logs: docker-compose -f $COMPOSE_FILE logs -f"
        echo "  - Stop services: docker-compose -f $COMPOSE_FILE down"
        echo "  - Restart: docker-compose -f $COMPOSE_FILE restart"
        echo ""
        
        # Wait a bit and check health
        info "Checking service health..."
        sleep 10
        
        if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
            log "✓ Watchtower dashboard is healthy"
        else
            warn "✗ Watchtower dashboard health check failed"
        fi
        
        if [ "$ENVIRONMENT" = "production" ]; then
            if curl -sf http://localhost:3000/api/health > /dev/null 2>&1; then
                log "✓ Grafana is healthy"
            else
                warn "✗ Grafana health check failed"
            fi
        fi
    fi
else
    error "Failed to start services"
    exit 1
fi

log "Setup complete!" 