/**
 * rtcEngine.js - Core WebRTC logic for P2P file transfer.
 * Handles roles, data channels, backpressure, and integrity.
 */

const CHUNK_SIZE = 64 * 1024; // 64KB
const BUFFER_THRESHOLD = 1 * 1024 * 1024; // 1MB backpressure threshold
const METADATA_TIMEOUT = 20000; // 20s

class RTCEngine {
    constructor(role, signaling, iceServers) {
        this.role = role; // 'initiator' or 'receiver'
        this.signaling = signaling;
        this.iceServers = iceServers;
        
        this.pc = null;
        this.dc = null;
        this.fileData = null;
        
        this.pendingCandidates = [];
        this.onStateChange = null;
        this.onProgress = null;
        this.onComplete = null;
        this.onFailed = null;
        
        this.receivedBytes = 0;
        this.receiveBuffer = [];
        this.metadata = null;
        this.metadataTimeout = null;

        this.init();
    }

    init() {
        console.log(`[RTC] Initializing as ${this.role}...`);
        this.pc = new RTCPeerConnection({ iceServers: this.iceServers });

        this.pc.onicecandidate = (e) => {
            if (e.candidate) {
                this.signaling.send({ type: 'candidate', data: e.candidate });
            }
        };

        this.pc.onconnectionstatechange = () => {
            console.log(`[RTC] Connection state: ${this.pc.connectionState}`);
            if (this.onStateChange) this.onStateChange(this.pc.connectionState);
            if (this.pc.connectionState === 'failed') {
                if (this.onFailed) this.onFailed('WebRTC connection failed. This usually happens due to restrictive NATs or firewalls.');
            }
        };

        if (this.role === 'initiator') {
            const channel = this.pc.createDataChannel('fileTransfer', { ordered: true });
            this.setupDataChannel(channel);
            this.createOffer();
        } else {
            this.pc.ondatachannel = (e) => {
                this.setupDataChannel(e.channel);
            };
        }
    }

    async createOffer() {
        try {
            const offer = await this.pc.createOffer();
            await this.pc.setLocalDescription(offer);
            this.signaling.send({ type: 'offer', data: offer });
        } catch (e) {
            console.error('[RTC] Create offer failed:', e);
            if (this.onFailed) this.onFailed('Failed to create RTC offer.');
        }
    }

    async handleSignalingMessage(msg) {
        try {
            if (msg.type === 'offer' && this.role === 'receiver') {
                await this.pc.setRemoteDescription(new RTCSessionDescription(msg.data));
                const answer = await this.pc.createAnswer();
                await this.pc.setLocalDescription(answer);
                this.signaling.send({ type: 'answer', data: answer });
                this.processPendingCandidates();
            } else if (msg.type === 'answer' && this.role === 'initiator') {
                await this.pc.setRemoteDescription(new RTCSessionDescription(msg.data));
                this.processPendingCandidates();
            } else if (msg.type === 'candidate') {
                if (this.pc.remoteDescription && this.pc.remoteDescription.type) {
                    await this.pc.addIceCandidate(new RTCIceCandidate(msg.data));
                } else {
                    this.pendingCandidates.push(msg.data);
                }
            }
        } catch (e) {
            console.error('[RTC] Signaling handling error:', e);
        }
    }

    async processPendingCandidates() {
        while (this.pendingCandidates.length > 0) {
            const candidate = this.pendingCandidates.shift();
            try {
                await this.pc.addIceCandidate(new RTCIceCandidate(candidate));
            } catch (e) {
                console.warn('[RTC] Failed to add pending candidate:', e);
            }
        }
    }

    setupDataChannel(dc) {
        this.dc = dc;
        this.dc.binaryType = 'arraybuffer';

        this.dc.onopen = () => {
            console.log('[RTC] DataChannel open.');
            if (this.role === 'receiver') {
                this.startMetadataTimeout();
                // Signal to sender that we are ready to receive metadata
                this.dc.send(JSON.stringify({ type: 'ready' }));
            }
        };

        this.dc.onmessage = (e) => this.handleDataMessage(e.data);
        
        this.dc.onclose = () => {
            console.warn('[RTC] DataChannel closed.');
            // Only fail if we haven't completed
            if (this.receivedBytes < (this.metadata?.size || Infinity)) {
                if (this.onFailed) this.onFailed('Data channel closed unexpectedly.');
            }
        };

        this.dc.onerror = (e) => {
            console.error('[RTC] DataChannel error:', e);
            if (this.onFailed) this.onFailed('Data channel error.');
        };
    }

    startMetadataTimeout() {
        if (this.metadataTimeout) clearTimeout(this.metadataTimeout);
        this.metadataTimeout = setTimeout(() => {
            if (!this.metadata) {
                console.error('[RTC] Metadata timeout.');
                if (this.onFailed) this.onFailed('Timed out waiting for file metadata.');
            }
        }, METADATA_TIMEOUT);
    }

    handleDataMessage(data) {
        if (typeof data === 'string') {
            try {
                const msg = JSON.parse(data);
                if (msg.type === 'metadata') {
                    console.log('[RTC] Metadata received:', msg);
                    this.metadata = msg;
                    clearTimeout(this.metadataTimeout);
                    this.dc.send(JSON.stringify({ type: 'metadata_ack' }));
                } else if (msg.type === 'ready' && this.role === 'initiator') {
                    // Receiver is ready, sender can send metadata if they have it
                    if (this.fileData) this.sendMetadata(this.fileData);
                } else if (msg.type === 'metadata_ack' && this.role === 'initiator') {
                    this.startTransfer();
                }
            } catch (e) {
                console.error('[RTC] Parse error in DataChannel:', e);
            }
        } else {
            // Binary chunk
            this.receivedBytes += data.byteLength;
            this.receiveBuffer.push(data);
            
            if (this.onProgress) {
                this.onProgress(this.receivedBytes, this.metadata ? this.metadata.size : 0);
            }

            if (this.metadata && this.receivedBytes >= this.metadata.size) {
                this.finalizeDownload();
            }
        }
    }

    sendMetadata(fileInfo) {
        this.fileData = fileInfo;
        if (this.dc && this.dc.readyState === 'open') {
            const msg = {
                type: 'metadata',
                name: fileInfo.name,
                size: fileInfo.size,
                mime: fileInfo.type
            };
            this.dc.send(JSON.stringify(msg));
        }
    }

    async startTransfer() {
        if (!this.fileData || !this.dc || this.dc.readyState !== 'open') return;
        console.log('[RTC] Starting transfer...');
        const file = this.fileData.file;
        let offset = 0;

        const readAndSend = () => {
            if (offset >= file.size) {
                console.log('[RTC] File sent completely.');
                if (this.onComplete) this.onComplete();
                return;
            }

            if (this.dc.bufferedAmount > BUFFER_THRESHOLD) {
                this.dc.onbufferedamountlow = () => {
                    this.dc.onbufferedamountlow = null;
                    readAndSend();
                };
                return;
            }

            const chunk = file.slice(offset, offset + CHUNK_SIZE);
            const reader = new FileReader();
            reader.onload = (e) => {
                if (this.dc.readyState === 'open') {
                    this.dc.send(e.target.result);
                    offset += e.target.result.byteLength;
                    if (this.onProgress) this.onProgress(offset, file.size);
                    readAndSend();
                }
            };
            reader.readAsArrayBuffer(chunk);
        };

        readAndSend();
    }

    finalizeDownload() {
        console.log('[RTC] Finalizing download...');
        const blob = new Blob(this.receiveBuffer, { type: this.metadata.mime });
        const url = URL.createObjectURL(blob);
        if (this.onComplete) this.onComplete(url, this.metadata.name);
        
        // Clean up buffer
        this.receiveBuffer = [];
    }

    close() {
        if (this.metadataTimeout) clearTimeout(this.metadataTimeout);
        if (this.dc) {
            this.dc.onclose = null;
            this.dc.close();
        }
        if (this.pc) {
            this.pc.onicecandidate = null;
            this.pc.onconnectionstatechange = null;
            this.pc.close();
        }
    }
}

window.RTCEngine = RTCEngine;
