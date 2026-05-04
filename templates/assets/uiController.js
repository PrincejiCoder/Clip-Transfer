/**
 * uiController.js - Coordinates storage, signaling, and WebRTC engines.
 * Manages UI state transitions and element updates.
 */

class UIController {
    constructor() {
        this.slug = window.location.pathname.split('/').pop();
        this.isSharePage = window.location.search.includes('created=true');
        this.isViewPage = !this.isSharePage && window.location.pathname.length > 1;
        this.isCreatePage = window.location.pathname === '/' || (this.isViewPage && document.getElementById('drop-form'));

        this.signaling = null;
        this.engine = null;
        this.startTime = 0;

        // UI Elements (Cached)
        this.els = {
            transferPanel: document.getElementById('transfer-panel'),
            status: document.getElementById('transfer-status'),
            fileInfo: document.getElementById('file-info'),
            fileName: document.getElementById('file-name'),
            fileSize: document.getElementById('file-size'),
            progressContainer: document.getElementById('progress-container'),
            progressBar: document.getElementById('progress-bar'),
            progressText: document.getElementById('progress-text'),
            speedText: document.getElementById('speed-text'),
            failureState: document.getElementById('failure-state'),
            errorMessage: document.getElementById('error-message'),
            retryBtn: document.getElementById('retry-button'),
            cancelBtn: document.getElementById('cancel-transfer'),
            receiverReady: document.getElementById('receiver-ready'),
            downloadLink: document.getElementById('download-link'),
            dropForm: document.getElementById('drop-form'),
            fileInput: document.getElementById('file-input'),
            attachBtn: document.getElementById('attach-button')
        };

        this.init();
    }

    async init() {
        if (this.isCreatePage) {
            this.setupCreateFlow();
        } else if (this.isSharePage) {
            this.setupShareFlow();
        } else if (this.isViewPage) {
            this.setupViewFlow();
        }

        // Global event listeners
        if (this.els.cancelBtn) {
            this.els.cancelBtn.onclick = () => this.cleanup();
        }
        if (this.els.retryBtn) {
            this.els.retryBtn.onclick = () => location.reload();
        }

        // Visibility warning
        document.addEventListener('visibilitychange', () => {
            if (document.hidden && this.engine && this.engine.dc && this.engine.dc.readyState === 'open') {
                console.warn('[UI] Tab backgrounded during active transfer. Performance may degrade.');
            }
        });
    }

    setupCreateFlow() {
        if (!this.els.dropForm || !this.els.fileInput) return;

        if (this.els.attachBtn) {
            this.els.attachBtn.onclick = () => this.els.fileInput.click();
        }

        this.els.fileInput.onchange = (e) => {
            const file = e.target.files[0];
            if (file) {
                if (file.size > 150 * 1024 * 1024) {
                    alert('File exceeds 150MB limit for P2P sharing.');
                    this.els.fileInput.value = '';
                    return;
                }
                // Update UI to show file is attached
                if (this.els.attachBtn) {
                    this.els.attachBtn.classList.add('text-cyan-400', 'border-cyan-400');
                    this.els.attachBtn.innerHTML = '<span class="material-symbols-outlined text-[20px]">check_circle</span>';
                }
            }
        };

        this.els.dropForm.onsubmit = async (e) => {
            const file = this.els.fileInput.files[0];
            if (file) {
                e.preventDefault();
                try {
                    const slug = this.els.dropForm.action.split('/').pop() || this.slug;
                    console.log(`[UI] Storing file for slug: ${slug}`);
                    await window.fileStore.saveFile(slug, file);
                    this.els.dropForm.submit();
                } catch (err) {
                    alert('Failed to prepare file for sharing: ' + err.message);
                }
            }
        };
    }

    async setupShareFlow() {
        try {
            const fileData = await window.fileStore.getFile(this.slug);
            if (!fileData) {
                console.log('[UI] No file found in IndexedDB for this slug. P2P mode disabled.');
                return;
            }

            this.showTransferPanel('Initializing Sender...');
            this.showFileInfo(fileData.name, fileData.size);

            await this.startP2P('initiator', fileData);
        } catch (err) {
            this.showError(err.message);
        }
    }

    async setupViewFlow() {
        // Receivers just connect and wait
        this.showTransferPanel('Connecting to Peer...');
        await this.startP2P('receiver', null);
    }

    async startP2P(role, fileData) {
        // 1. Fetch ICE Servers (TURN/STUN)
        let iceServers = [{ urls: 'stun:stun.l.google.com:19302' }];
        try {
            const res = await fetch('/api/webrtc/turn-credentials');
            if (res.ok) iceServers = await res.json();
        } catch (e) {
            console.warn('[UI] Failed to fetch TURN credentials, falling back to STUN.', e);
        }

        // 2. Initialize Signaling
        this.signaling = new window.SignalingClient(this.slug);
        
        // 3. Initialize Engine
        this.engine = new window.RTCEngine(role, this.signaling, iceServers);
        if (fileData) this.engine.fileData = fileData;

        // 4. Connect Signaling
        this.signaling.onMessage = (msg) => {
            if (msg.type === 'session_expired') {
                this.showError('This sharing session has expired (30m limit).');
                return;
            }
            this.engine.handleSignalingMessage(msg);
        };

        this.signaling.onDisconnect = () => {
            if (this.engine && this.engine.pc.connectionState !== 'connected') {
                this.showStatus('Signaling lost. Reconnecting...');
            }
        };

        this.signaling.onReconnectFailed = () => {
            this.showError('Failed to reconnect to signaling server.');
        };

        this.signaling.connect();

        // 5. Engine Hooks
        this.engine.onStateChange = (state) => {
            switch(state) {
                case 'connecting': this.showStatus('Establishing P2P Connection...'); break;
                case 'connected': this.showStatus('Connected. Negotiating...'); break;
                case 'failed': this.showError('P2P connection failed. Peer might be behind a restrictive firewall.'); break;
                case 'disconnected': 
                    if (this.engine.receivedBytes < (this.engine.metadata?.size || Infinity)) {
                        this.showStatus('Peer disconnected.'); 
                    }
                    break;
            }
        };

        this.engine.onProgress = (current, total) => {
            if (this.startTime === 0) this.startTime = Date.now();
            this.updateProgress(current, total);
        };

        this.engine.onComplete = (url, name) => {
            this.showStatus('Transfer Complete');
            this.els.progressContainer.classList.add('hidden');
            if (url) {
                this.els.receiverReady.classList.remove('hidden');
                this.els.downloadLink.href = url;
                this.els.downloadLink.download = name;
            }
            // Cleanup storage if we are sender and transfer is done
            if (role === 'initiator') {
                window.fileStore.deleteFile(this.slug);
            }
        };

        this.engine.onFailed = (msg) => this.showError(msg);
    }

    showTransferPanel(status) {
        if (this.els.transferPanel) {
            this.els.transferPanel.classList.remove('hidden');
            this.showStatus(status);
        }
    }

    showStatus(text) {
        if (this.els.status) this.els.status.innerText = text;
    }

    showFileInfo(name, size) {
        if (this.els.fileInfo) {
            this.els.fileInfo.classList.remove('hidden');
            this.els.fileName.innerText = name;
            this.els.fileSize.innerText = this.formatSize(size);
        }
    }

    updateProgress(current, total) {
        if (!this.els.progressContainer) return;
        this.els.progressContainer.classList.remove('hidden');
        
        const percent = total > 0 ? Math.min(100, Math.round((current / total) * 100)) : 0;
        if (this.els.progressBar) this.els.progressBar.style.width = `${percent}%`;
        if (this.els.progressText) this.els.progressText.innerText = `${percent}%`;
        
        const elapsed = (Date.now() - this.startTime) / 1000;
        if (elapsed > 0 && this.els.speedText) {
            this.els.speedText.innerText = `${this.formatSize(current / elapsed)}/s`;
        }
    }

    showError(msg) {
        this.showStatus('Error');
        if (this.els.errorMessage) this.els.errorMessage.innerText = msg;
        if (this.els.failureState) this.els.failureState.classList.remove('hidden');
        if (this.els.progressContainer) this.els.progressContainer.classList.add('hidden');
        this.cleanup();
    }

    formatSize(bytes) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    cleanup() {
        if (this.engine) this.engine.close();
        if (this.signaling) this.signaling.close();
        if (this.els.transferPanel && this.isViewPage && !this.els.receiverReady.classList.contains('hidden')) {
            // Keep panel if finished
        } else if (this.els.transferPanel) {
            this.els.transferPanel.classList.add('hidden');
        }
    }
}

// Initialize on DOM load
window.addEventListener('DOMContentLoaded', () => {
    window.uiController = new UIController();
});
