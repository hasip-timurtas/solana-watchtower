# Solana Watchtower Docker Deployment

This directory contains Docker configuration files for deploying Solana Watchtower in containerized environments.

## Quick Start

### Prerequisites

- Docker 20.10+
- Docker Compose 2.0+
- 4GB+ RAM available
- 10GB+ disk space

### Basic Deployment

1. **Copy environment configuration:**
   ```bash
   cp docker/env.example .env
   ```

2. **Edit environment variables:**
   ```bash
   nano .env
   ```
   
   Configure at least:
   - `SOLANA_RPC_URL` - Your Solana RPC endpoint
   - `SOLANA_WS_URL` - Your Solana WebSocket endpoint
   - Email/Telegram/Slack credentials for notifications

3. **Start the services:**
   ```bash
   docker-compose -f docker/docker-compose.yml up -d
   ```

4. **Access the dashboard:**
   - Watchtower Dashboard: http://localhost:8080
   - Grafana: http://localhost:3000 (admin/admin)
   - Prometheus: http://localhost:9091

## Configuration

### Environment Variables

Copy `docker/env.example` to `.env` and configure:

#### Required Settings
- `SOLANA_RPC_URL`: Solana RPC endpoint
- `SOLANA_WS_URL`: Solana WebSocket endpoint

#### Notification Settings
Configure at least one notification channel:
- **Email**: `SMTP_HOST`, `SMTP_USERNAME`, `SMTP_PASSWORD`
- **Telegram**: `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`
- **Slack**: `SLACK_WEBHOOK_URL`
- **Discord**: `DISCORD_WEBHOOK_URL`

#### Optional Settings
- `RUST_LOG`: Log level (debug, info, warn, error)
- `GRAFANA_PASSWORD`: Grafana admin password
- `METRICS_RETENTION_DAYS`: How long to keep metrics

### Configuration Files

Mount your configuration files:
```yaml
volumes:
  - ./your-config.toml:/app/configs/watchtower.toml:ro
```

## Service Architecture

### Core Services

1. **watchtower**: Main monitoring application
   - Dashboard (port 8080)
   - Metrics endpoint (port 9090)
   - WebSocket monitoring
   - Alert processing

2. **redis**: Caching and rate limiting
   - Used for alert deduplication
   - Rate limiting notifications

3. **prometheus**: Metrics storage
   - Scrapes metrics from watchtower
   - 7-day retention by default

4. **grafana**: Visualization
   - Pre-configured dashboards
   - Connects to Prometheus

5. **nginx**: Reverse proxy (optional, production profile)
   - SSL termination
   - Load balancing
   - Static file serving

## Deployment Options

### Development

Start with basic services:
```bash
docker-compose -f docker/docker-compose.yml up watchtower redis
```

### Production

Use the production profile with nginx:
```bash
docker-compose -f docker/docker-compose.yml --profile production up -d
```

### Custom Builds

Build from source:
```bash
docker-compose -f docker/docker-compose.yml build watchtower
```

## Service Commands

### Run Different Services

The container supports multiple service modes:

1. **Dashboard only** (default):
   ```bash
   docker run -d solana-watchtower
   ```

2. **Engine only**:
   ```bash
   docker run -d solana-watchtower engine
   ```

3. **Both services**:
   ```bash
   docker run -d solana-watchtower all
   ```

4. **Validate configuration**:
   ```bash
   docker run --rm solana-watchtower validate
   ```

5. **Test notifications**:
   ```bash
   docker run --rm solana-watchtower test-notifications
   ```

## Monitoring and Logs

### View Logs
```bash
# All services
docker-compose -f docker/docker-compose.yml logs -f

# Specific service
docker-compose -f docker/docker-compose.yml logs -f watchtower
```

### Health Checks
```bash
# Check service health
docker-compose -f docker/docker-compose.yml ps

# Manual health check
curl http://localhost:8080/health
```

### Metrics Access
- Application metrics: http://localhost:9090/metrics
- Prometheus UI: http://localhost:9091
- Grafana dashboards: http://localhost:3000

## Data Persistence

All important data is stored in Docker volumes:

- `watchtower_data`: Application database and state
- `watchtower_logs`: Application logs
- `redis_data`: Redis persistence
- `prometheus_data`: Metrics storage
- `grafana_data`: Grafana configuration and dashboards

### Backup Data
```bash
# Create backup
docker run --rm -v watchtower_data:/data -v $(pwd):/backup alpine tar czf /backup/watchtower-backup.tar.gz -C /data .

# Restore backup
docker run --rm -v watchtower_data:/data -v $(pwd):/backup alpine tar xzf /backup/watchtower-backup.tar.gz -C /data
```

## Scaling and Performance

### Resource Requirements

**Minimum:**
- 2 CPU cores
- 4GB RAM
- 10GB disk

**Recommended:**
- 4+ CPU cores
- 8GB+ RAM
- 50GB+ SSD

### Scaling Options

1. **Horizontal scaling**: Run multiple watchtower instances
2. **Resource limits**: Configure in docker-compose.yml
3. **Connection pooling**: Adjust `MAX_CONNECTIONS` environment variable

## Troubleshooting

### Common Issues

1. **Connection to Solana RPC fails**:
   - Check `SOLANA_RPC_URL` and `SOLANA_WS_URL`
   - Verify network connectivity
   - Check RPC endpoint rate limits

2. **Notifications not working**:
   - Run `docker run --rm solana-watchtower test-notifications`
   - Check notification service credentials
   - Verify network access to notification services

3. **High memory usage**:
   - Reduce `EVENT_BUFFER_SIZE`
   - Lower `METRICS_RETENTION_DAYS`
   - Restart Redis periodically

4. **Dashboard not accessible**:
   - Check if port 8080 is available
   - Verify firewall settings
   - Check Docker network configuration

### Debug Mode

Enable debug logging:
```bash
RUST_LOG=debug docker-compose -f docker/docker-compose.yml up
```

### Service Restart

Restart specific services:
```bash
docker-compose -f docker/docker-compose.yml restart watchtower
```

## Security Considerations

### Production Checklist

- [ ] Change default Grafana password
- [ ] Configure proper SSL certificates
- [ ] Enable nginx reverse proxy
- [ ] Set up firewall rules
- [ ] Configure proper backup strategy
- [ ] Use secure RPC endpoints
- [ ] Enable monitoring alerts

### Network Security

The default configuration exposes:
- 8080: Dashboard (should be behind reverse proxy in production)
- 3000: Grafana (restrict access)
- 9091: Prometheus (restrict access)

Use nginx profile for production deployments with proper SSL.

## Updates and Maintenance

### Update Images
```bash
docker-compose -f docker/docker-compose.yml pull
docker-compose -f docker/docker-compose.yml up -d
```

### Clean Up
```bash
# Remove old images
docker image prune -f

# Remove unused volumes (be careful!)
docker volume prune
```

## Support

For issues and questions:
- Check the main project README
- Review logs: `docker-compose logs`
- Open an issue on GitHub 