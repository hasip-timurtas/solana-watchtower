#!/usr/bin/env python3
"""
Solana Watchtower Webhook Receiver Example

This script demonstrates how to receive and process webhook alerts
from Solana Watchtower and forward them to other systems.
"""

import json
import logging
import hmac
import hashlib
from datetime import datetime
from flask import Flask, request, jsonify
import requests

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Configuration
WEBHOOK_SECRET = "your-webhook-secret-here"
SLACK_WEBHOOK_URL = "https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK"
DISCORD_WEBHOOK_URL = "https://discord.com/api/webhooks/YOUR/DISCORD/WEBHOOK"

class AlertProcessor:
    """Process and route incoming alerts from Solana Watchtower"""
    
    def __init__(self):
        self.high_value_threshold = 1000  # SOL
        self.critical_programs = [
            "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo",  # Solend
            "mv3ekLzLbnVPNxjSKvqBpU3ZeZXPQdEC3bp5MDEBG68",  # Mango
        ]
    
    def process_alert(self, alert_data):
        """Process incoming alert and determine routing"""
        try:
            alert = Alert(alert_data)
            
            # Log all alerts
            logger.info(f"Received alert: {alert.rule_name} - {alert.severity}")
            
            # Route based on severity and type
            if alert.severity in ["critical", "high"]:
                self.handle_critical_alert(alert)
            elif alert.is_high_value_transaction():
                self.handle_high_value_alert(alert)
            elif alert.program_id in self.critical_programs:
                self.handle_protocol_alert(alert)
            
            # Store alert in database
            self.store_alert(alert)
            
            return {"status": "processed", "alert_id": alert.id}
            
        except Exception as e:
            logger.error(f"Error processing alert: {str(e)}")
            return {"status": "error", "message": str(e)}
    
    def handle_critical_alert(self, alert):
        """Handle critical severity alerts with immediate notifications"""
        message = f"🚨 CRITICAL ALERT: {alert.rule_name}\n"
        message += f"Program: {alert.program_name}\n"
        message += f"Description: {alert.description}\n"
        message += f"Transaction: https://solscan.io/tx/{alert.signature}"
        
        # Send to multiple channels for critical alerts
        self.send_to_slack(message, urgent=True)
        self.send_to_discord(message)
        
        # Send to PagerDuty or similar alerting system
        self.trigger_pagerduty_alert(alert)
    
    def handle_high_value_alert(self, alert):
        """Handle high-value transaction alerts"""
        amount_sol = alert.get_metadata("amount_sol", "Unknown")
        message = f"💰 High Value Transaction: {amount_sol} SOL\n"
        message += f"Rule: {alert.rule_name}\n"
        message += f"Transaction: https://solscan.io/tx/{alert.signature}"
        
        self.send_to_slack(message)
    
    def handle_protocol_alert(self, alert):
        """Handle alerts from critical DeFi protocols"""
        message = f"⚠️ Protocol Alert: {alert.program_name}\n"
        message += f"Rule: {alert.rule_name}\n"
        message += f"Severity: {alert.severity}\n"
        message += f"Description: {alert.description}"
        
        self.send_to_discord(message)
    
    def send_to_slack(self, message, urgent=False):
        """Send alert to Slack"""
        try:
            payload = {
                "text": message,
                "username": "Solana Watchtower",
                "icon_emoji": ":warning:" if urgent else ":information_source:"
            }
            
            if urgent:
                payload["channel"] = "#critical-alerts"
            
            response = requests.post(SLACK_WEBHOOK_URL, json=payload)
            response.raise_for_status()
            
        except Exception as e:
            logger.error(f"Failed to send Slack message: {str(e)}")
    
    def send_to_discord(self, message):
        """Send alert to Discord"""
        try:
            payload = {
                "content": message,
                "username": "Solana Watchtower"
            }
            
            response = requests.post(DISCORD_WEBHOOK_URL, json=payload)
            response.raise_for_status()
            
        except Exception as e:
            logger.error(f"Failed to send Discord message: {str(e)}")
    
    def trigger_pagerduty_alert(self, alert):
        """Trigger PagerDuty alert for critical issues"""
        # Placeholder for PagerDuty integration
        logger.info(f"Would trigger PagerDuty for: {alert.rule_name}")
    
    def store_alert(self, alert):
        """Store alert in database for historical analysis"""
        # Placeholder for database storage
        logger.info(f"Storing alert: {alert.id}")

class Alert:
    """Alert data model"""
    
    def __init__(self, data):
        self.raw_data = data
        self.id = data.get("id")
        self.rule_name = data.get("rule_name")
        self.severity = data.get("severity")
        self.program_id = data.get("program_id")
        self.program_name = data.get("program_name")
        self.description = data.get("description")
        self.signature = data.get("signature")
        self.metadata = data.get("metadata", {})
        self.timestamp = data.get("timestamp")
    
    def is_high_value_transaction(self):
        """Check if this is a high-value transaction"""
        amount_sol = self.get_metadata("amount_sol")
        if amount_sol:
            try:
                return float(amount_sol) > 1000  # 1000 SOL threshold
            except ValueError:
                return False
        return False
    
    def get_metadata(self, key, default=None):
        """Get metadata value by key"""
        return self.metadata.get(key, default)

def verify_webhook_signature(payload_body, signature):
    """Verify webhook signature for security"""
    expected_signature = hmac.new(
        WEBHOOK_SECRET.encode('utf-8'),
        payload_body,
        hashlib.sha256
    ).hexdigest()
    
    return hmac.compare_digest(f"sha256={expected_signature}", signature)

# Initialize alert processor
processor = AlertProcessor()

@app.route('/webhook/alerts', methods=['POST'])
def receive_alert():
    """Receive webhook alerts from Solana Watchtower"""
    try:
        # Verify signature if present
        signature = request.headers.get('X-Watchtower-Signature')
        if signature and WEBHOOK_SECRET:
            if not verify_webhook_signature(request.data, signature):
                logger.warning("Invalid webhook signature")
                return jsonify({"error": "Invalid signature"}), 401
        
        # Process the alert
        alert_data = request.json
        result = processor.process_alert(alert_data)
        
        return jsonify(result), 200
        
    except Exception as e:
        logger.error(f"Webhook error: {str(e)}")
        return jsonify({"error": "Internal server error"}), 500

@app.route('/health', methods=['GET'])
def health_check():
    """Health check endpoint"""
    return jsonify({
        "status": "healthy",
        "timestamp": datetime.utcnow().isoformat()
    })

@app.route('/stats', methods=['GET'])
def get_stats():
    """Get processing statistics"""
    # Placeholder for real statistics
    return jsonify({
        "alerts_processed": 1234,
        "last_alert": "2024-01-15T12:00:00Z",
        "uptime": "5d 2h 30m"
    })

if __name__ == '__main__':
    logger.info("Starting Solana Watchtower webhook receiver...")
    app.run(host='0.0.0.0', port=5000, debug=False) 