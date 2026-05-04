/**
 * signalingClient.js - Robust WebSocket signaling client.
 * Handles heartbeats, reconnection, and protocol mapping.
 */

class SignalingClient {
    constructor(slug) {
        this.slug = slug;
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 3;
        this.heartbeatTimer = null;
        this.pongReceived = true;
        this.onMessage = null;
        this.onDisconnect = null;
        this.onReconnectFailed = null;
        this.isExplicitlyClosed = false;
    }

    connect() {
        this.isExplicitlyClosed = false;
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws/${this.slug}`;
        
        console.log(`[Signaling] Connecting to ${wsUrl}...`);
        this.ws = new WebSocket(wsUrl);

        this.ws.onopen = () => {
            console.log('[Signaling] Connected.');
            this.reconnectAttempts = 0;
            this.startHeartbeat();
        };

        this.ws.onmessage = (event) => {
            try {
                const msg = JSON.parse(event.data);
                if (msg.type === 'pong') {
                    this.pongReceived = true;
                    return;
                }
                if (this.onMessage) this.onMessage(msg);
            } catch (e) {
                console.error('[Signaling] Failed to parse message:', event.data, e);
            }
        };

        this.ws.onclose = (event) => {
            this.stopHeartbeat();
            if (this.isExplicitlyClosed) return;

            console.warn(`[Signaling] Disconnected: ${event.code}`);
            if (this.onDisconnect) this.onDisconnect();
            this.handleReconnect();
        };

        this.ws.onerror = (error) => {
            console.error('[Signaling] Error:', error);
        };
    }

    send(msg) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(msg));
        } else {
            console.warn('[Signaling] Attempted to send message while disconnected:', msg.type);
        }
    }

    handleReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            const delay = Math.pow(2, this.reconnectAttempts) * 1000;
            console.log(`[Signaling] Reconnecting in ${delay}ms (Attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})...`);
            
            setTimeout(() => this.connect(), delay);
        } else {
            console.error('[Signaling] Max reconnect attempts reached.');
            if (this.onReconnectFailed) this.onReconnectFailed();
        }
    }

    startHeartbeat() {
        this.stopHeartbeat();
        this.pongReceived = true;
        this.heartbeatTimer = setInterval(() => {
            if (!this.pongReceived) {
                console.warn('[Signaling] Heartbeat timeout (30s). Triggering reconnect...');
                if (this.ws) this.ws.close();
                return;
            }
            this.pongReceived = false;
            // The server is configured to respond to 'ping' with a 'pong' text message
            this.send({ type: 'ping' });
        }, 15000); // 15s check
    }

    stopHeartbeat() {
        if (this.heartbeatTimer) {
            clearInterval(this.heartbeatTimer);
            this.heartbeatTimer = null;
        }
    }

    close() {
        this.isExplicitlyClosed = true;
        this.stopHeartbeat();
        if (this.ws) this.ws.close();
    }
}

window.SignalingClient = SignalingClient;
