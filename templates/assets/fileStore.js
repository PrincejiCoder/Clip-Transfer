/**
 * fileStore.js - IndexedDB module for temporary file persistence.
 * Bridges create.html and share.html.
 */

const DB_NAME = 'LinkDropDB';
const DB_VERSION = 1;
const STORE_NAME = 'files';
const TTL_HOURS = 1; // 1 hour TTL for local storage

class FileStore {
    constructor() {
        this.db = null;
        this.initPromise = this.init();
    }

    async init() {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open(DB_NAME, DB_VERSION);
            request.onupgradeneeded = (e) => {
                const db = e.target.result;
                if (!db.objectStoreNames.contains(STORE_NAME)) {
                    db.createObjectStore(STORE_NAME, { keyPath: 'slug' });
                }
            };
            request.onsuccess = (e) => {
                this.db = e.target.result;
                this.cleanup();
                resolve();
            };
            request.onerror = (e) => reject(e.target.error);
        });
    }

    async saveFile(slug, file) {
        await this.initPromise;
        const sessionId = crypto.randomUUID ? crypto.randomUUID() : Math.random().toString(36).substring(2);
        const data = {
            slug,
            file,
            name: file.name,
            size: file.size,
            type: file.type,
            timestamp: Date.now(),
            sessionId
        };

        // Enforce 150MB limit
        if (file.size > 150 * 1024 * 1024) {
            throw new Error('File exceeds 150MB limit.');
        }

        return new Promise((resolve, reject) => {
            const tx = this.db.transaction(STORE_NAME, 'readwrite');
            const store = tx.objectStore(STORE_NAME);
            const request = store.put(data);
            request.onsuccess = () => {
                sessionStorage.setItem(`linkdrop_session_${slug}`, sessionId);
                resolve(data);
            };
            request.onerror = (e) => reject(e.target.error);
        });
    }

    async getFile(slug) {
        await this.initPromise;
        return new Promise((resolve, reject) => {
            const tx = this.db.transaction(STORE_NAME, 'readonly');
            const store = tx.objectStore(STORE_NAME);
            const request = store.get(slug);
            request.onsuccess = () => {
                const data = request.result;
                if (!data) return resolve(null);

                // Session ID Validation
                const currentSessionId = sessionStorage.getItem(`linkdrop_session_${slug}`);
                if (data.sessionId !== currentSessionId) {
                    return reject(new Error('This session is already active in another tab or the browser was closed.'));
                }

                resolve(data);
            };
            request.onerror = (e) => reject(e.target.error);
        });
    }

    async deleteFile(slug) {
        await this.initPromise;
        return new Promise((resolve, reject) => {
            const tx = this.db.transaction(STORE_NAME, 'readwrite');
            const store = tx.objectStore(STORE_NAME);
            const request = store.delete(slug);
            request.onsuccess = () => {
                sessionStorage.removeItem(`linkdrop_session_${slug}`);
                resolve();
            };
            request.onerror = (e) => reject(e.target.error);
        });
    }

    async cleanup() {
        if (!this.db) return;
        const tx = this.db.transaction(STORE_NAME, 'readwrite');
        const store = tx.objectStore(STORE_NAME);
        const now = Date.now();
        const cursorRequest = store.openCursor();
        cursorRequest.onsuccess = (e) => {
            const cursor = e.target.result;
            if (cursor) {
                const data = cursor.value;
                if (now - data.timestamp > TTL_HOURS * 60 * 60 * 1000) {
                    store.delete(cursor.key);
                }
                cursor.continue();
            }
        };
    }
}

window.fileStore = new FileStore();
