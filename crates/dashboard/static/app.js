// Solana Watchtower Dashboard JavaScript

class WatchtowerDashboard {
    constructor() {
        this.websocket = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 5;
        this.reconnectDelay = 1000;
        this.charts = {};
        this.lastUpdate = Date.now();
        
        this.init();
    }

    init() {
        this.connectWebSocket();
        this.setupEventListeners();
        this.updateConnectionStatus();
        
        // Auto-refresh data every 30 seconds if WebSocket is not available
        setInterval(() => {
            if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
                this.refreshPageData();
            }
        }, 30000);
    }

    connectWebSocket() {
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws`;
        
        try {
            this.websocket = new WebSocket(wsUrl);
            
            this.websocket.onopen = () => {
                console.log('WebSocket connected');
                this.reconnectAttempts = 0;
                this.updateConnectionStatus(true);
                
                // Send initial ping
                this.sendMessage({ type: 'Ping' });
            };
            
            this.websocket.onmessage = (event) => {
                this.handleWebSocketMessage(event);
            };
            
            this.websocket.onclose = () => {
                console.log('WebSocket disconnected');
                this.updateConnectionStatus(false);
                this.scheduleReconnect();
            };
            
            this.websocket.onerror = (error) => {
                console.error('WebSocket error:', error);
                this.updateConnectionStatus(false);
            };
            
        } catch (error) {
            console.error('Failed to create WebSocket connection:', error);
            this.updateConnectionStatus(false);
            this.scheduleReconnect();
        }
    }

    scheduleReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);
            
            console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
            
            setTimeout(() => {
                this.connectWebSocket();
            }, delay);
        } else {
            console.error('Max reconnection attempts reached');
            this.showNotification('Connection lost. Please refresh the page.', 'error');
        }
    }

    handleWebSocketMessage(event) {
        try {
            const message = JSON.parse(event.data);
            this.lastUpdate = Date.now();
            
            switch (message.type) {
                case 'Ping':
                    this.sendMessage({ type: 'Pong' });
                    break;
                    
                case 'Alert':
                    this.handleNewAlert(message.data);
                    break;
                    
                case 'Status':
                    this.handleStatusUpdate(message.data);
                    break;
                    
                case 'Metrics':
                    this.handleMetricsUpdate(message.data);
                    break;
                    
                case 'Error':
                    console.error('Server error:', message.message);
                    this.showNotification(message.message, 'error');
                    break;
                    
                default:
                    console.log('Unknown message type:', message.type);
            }
        } catch (error) {
            console.error('Error parsing WebSocket message:', error);
        }
    }

    sendMessage(message) {
        if (this.websocket && this.websocket.readyState === WebSocket.OPEN) {
            this.websocket.send(JSON.stringify(message));
        }
    }

    updateConnectionStatus(connected = false) {
        const statusElement = document.getElementById('connection-status');
        if (statusElement) {
            const icon = statusElement.querySelector('i');
            const text = statusElement.childNodes[1];
            
            if (connected) {
                icon.style.color = '#22c55e';
                text.textContent = ' Connected';
                statusElement.title = 'Real-time connection active';
            } else {
                icon.style.color = '#ef4444';
                text.textContent = ' Disconnected';
                statusElement.title = 'Connection lost - some data may be stale';
            }
        }
    }

    handleNewAlert(alertData) {
        // Show browser notification if permitted
        this.showBrowserNotification(alertData);
        
        // Show in-app notification
        this.showNotification(
            `New ${alertData.severity} alert: ${alertData.message}`,
            alertData.severity.toLowerCase()
        );
        
        // Update alert badge count
        this.updateAlertBadge();
        
        // If on alerts page, add to list
        if (window.location.pathname === '/alerts') {
            this.addAlertToList(alertData);
        }
    }

    handleStatusUpdate(statusData) {
        // Update dashboard status displays
        this.updateDashboardStatus(statusData);
    }

    handleMetricsUpdate(metricsData) {
        // Update charts and metrics displays
        this.updateMetricsCharts(metricsData);
    }

    showBrowserNotification(alertData) {
        if ('Notification' in window && Notification.permission === 'granted') {
            new Notification(`Solana Watchtower - ${alertData.severity} Alert`, {
                body: alertData.message,
                icon: '/static/favicon.ico',
                tag: alertData.id
            });
        }
    }

    showNotification(message, type = 'info') {
        // Create notification element
        const notification = document.createElement('div');
        notification.className = `notification notification-${type}`;
        notification.innerHTML = `
            <div class="notification-content">
                <span class="notification-message">${message}</span>
                <button class="notification-close" onclick="this.parentElement.parentElement.remove()">
                    <i class="fas fa-times"></i>
                </button>
            </div>
        `;
        
        // Add to page
        let container = document.getElementById('notifications');
        if (!container) {
            container = document.createElement('div');
            container.id = 'notifications';
            container.className = 'notifications-container';
            document.body.appendChild(container);
        }
        
        container.appendChild(notification);
        
        // Auto-remove after 5 seconds
        setTimeout(() => {
            if (notification.parentElement) {
                notification.remove();
            }
        }, 5000);
    }

    setupEventListeners() {
        // Request notification permission
        if ('Notification' in window && Notification.permission === 'default') {
            Notification.requestPermission();
        }
        
        // Global keyboard shortcuts
        document.addEventListener('keydown', (event) => {
            if (event.ctrlKey || event.metaKey) {
                switch (event.key) {
                    case 'r':
                        event.preventDefault();
                        this.refreshPageData();
                        break;
                    case '/':
                        event.preventDefault();
                        this.focusSearch();
                        break;
                }
            }
        });
        
        // Page visibility change handling
        document.addEventListener('visibilitychange', () => {
            if (document.visibilityState === 'visible') {
                // Refresh data when page becomes visible
                this.refreshPageData();
            }
        });
    }

    refreshPageData() {
        // Refresh current page data
        if (typeof refreshAlerts === 'function') {
            refreshAlerts();
        }
        
        // Fetch latest status
        this.fetchSystemStatus();
    }

    async fetchSystemStatus() {
        try {
            const response = await fetch('/api/status');
            const data = await response.json();
            
            if (data.success) {
                this.updateDashboardStatus(data.data);
            }
        } catch (error) {
            console.error('Failed to fetch system status:', error);
        }
    }

    updateDashboardStatus(statusData) {
        // Update various status elements on the page
        const elements = {
            'engine-status': statusData.engine_status,
            'alert-count': statusData.alert_count,
            'active-rules': statusData.active_rules,
            'uptime': this.formatUptime(statusData.uptime_seconds),
            'memory-usage': `${statusData.memory_usage_mb} MB`,
            'websocket-connections': statusData.connected_websockets
        };
        
        Object.entries(elements).forEach(([id, value]) => {
            const element = document.getElementById(id);
            if (element) {
                element.textContent = value;
            }
        });
    }

    updateMetricsCharts(metricsData) {
        // Update Chart.js charts if they exist
        Object.values(this.charts).forEach(chart => {
            if (chart && typeof chart.update === 'function') {
                try {
                    // Add new data point
                    const now = new Date(metricsData.timestamp * 1000);
                    
                    // Prevent duplicate timestamps
                    if (chart.data.labels.length > 0 && 
                        chart.data.labels[chart.data.labels.length - 1] === now.toLocaleTimeString()) {
                        return;
                    }
                    
                    chart.data.labels.push(now.toLocaleTimeString());
                    
                    chart.data.datasets.forEach((dataset, index) => {
                        const metricName = dataset.label.toLowerCase().replace(/\s+/g, '_');
                        const value = metricsData.metrics[metricName] || 0;
                        dataset.data.push(value);
                    });
                    
                    // Keep only last 20 data points
                    if (chart.data.labels.length > 20) {
                        chart.data.labels.shift();
                        chart.data.datasets.forEach(dataset => {
                            dataset.data.shift();
                        });
                    }
                    
                    chart.update('none'); // No animation for real-time updates
                } catch (error) {
                    console.error('Error updating chart:', error);
                }
            }
        });
    }

    formatUptime(seconds) {
        const days = Math.floor(seconds / 86400);
        const hours = Math.floor((seconds % 86400) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        
        if (days > 0) {
            return `${days}d ${hours}h ${minutes}m`;
        } else if (hours > 0) {
            return `${hours}h ${minutes}m`;
        } else {
            return `${minutes}m`;
        }
    }

    updateAlertBadge() {
        // Update alert count badge in navigation
        const badge = document.querySelector('.alert-badge');
        if (badge) {
            const currentCount = parseInt(badge.textContent) || 0;
            badge.textContent = currentCount + 1;
            badge.style.display = 'inline';
        }
    }

    addAlertToList(alertData) {
        // Add new alert to alerts list if on alerts page
        if (typeof addNewAlert === 'function') {
            addNewAlert(alertData);
        }
    }

    focusSearch() {
        const searchInput = document.querySelector('input[type="search"], .search-input');
        if (searchInput) {
            searchInput.focus();
        }
    }
}

// Utility functions
window.formatTimestamp = function(timestamp) {
    return new Date(timestamp).toLocaleString();
};

window.formatNumber = function(num) {
    return new Intl.NumberFormat().format(num);
};

window.copyToClipboard = function(text) {
    navigator.clipboard.writeText(text).then(() => {
        dashboard.showNotification('Copied to clipboard', 'success');
    }).catch(err => {
        console.error('Failed to copy:', err);
        dashboard.showNotification('Failed to copy to clipboard', 'error');
    });
};

// Initialize dashboard when DOM is loaded
let dashboard;
document.addEventListener('DOMContentLoaded', function() {
    dashboard = new WatchtowerDashboard();
    window.dashboard = dashboard; // Make available globally
});

// CSS for notifications (injected dynamically)
const notificationStyles = `
    .notifications-container {
        position: fixed;
        top: 20px;
        right: 20px;
        z-index: 1000;
        max-width: 400px;
    }
    
    .notification {
        background: white;
        border-radius: 8px;
        box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        margin-bottom: 10px;
        border-left: 4px solid;
        animation: slideIn 0.3s ease-out;
    }
    
    .notification-info { border-left-color: #3b82f6; }
    .notification-success { border-left-color: #22c55e; }
    .notification-warning { border-left-color: #f59e0b; }
    .notification-error { border-left-color: #ef4444; }
    .notification-critical { border-left-color: #dc2626; }
    .notification-high { border-left-color: #ea580c; }
    .notification-medium { border-left-color: #d97706; }
    .notification-low { border-left-color: #65a30d; }
    
    .notification-content {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 12px 16px;
    }
    
    .notification-message {
        flex: 1;
        font-size: 14px;
        color: #374151;
    }
    
    .notification-close {
        background: none;
        border: none;
        cursor: pointer;
        color: #6b7280;
        padding: 4px;
        margin-left: 12px;
    }
    
    .notification-close:hover {
        color: #374151;
    }
    
    @keyframes slideIn {
        from {
            transform: translateX(100%);
            opacity: 0;
        }
        to {
            transform: translateX(0);
            opacity: 1;
        }
    }
`;

// Inject notification styles
const styleSheet = document.createElement('style');
styleSheet.textContent = notificationStyles;
document.head.appendChild(styleSheet); 