// Note IndexedDB Operations
// This module handles all note-related database operations

// Helper function to convert Uint8Array to base64 string
function uint8ArrayToBase64(uint8Array) {
    let binary = '';
    const len = uint8Array.byteLength;
    for (let i = 0; i < len; i++) {
        binary += String.fromCharCode(uint8Array[i]);
    }
    return btoa(binary);
}

// Helper function to convert base64 string to Uint8Array
function base64ToUint8Array(base64) {
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
}

// Helper function to get database
async function getDatabase() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open('miden_transport_client', 2);
        request.onsuccess = () => {
            resolve(request.result);
        };
        request.onerror = () => {
            reject(new Error('Failed to open database'));
        };
    });
}

// Store a note
export function storeNote(noteId, header, details, createdAt) {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const transaction = db.transaction(['stored_notes'], 'readwrite');
            const store = transaction.objectStore('stored_notes');
            
            // Convert Uint8Arrays to base64 for storage
            const noteData = {
                noteId: uint8ArrayToBase64(noteId),
                header: uint8ArrayToBase64(header),
                details: uint8ArrayToBase64(details),
                createdAt: createdAt,
                tag: 0 // Will be extracted from header if needed
            };
            
            const request = store.put(noteData);
            request.onsuccess = () => resolve(undefined);
            request.onerror = () => reject(new Error('Failed to store note'));
        } catch (error) {
            reject(error);
        }
    });
}

// Get a stored note by ID
export function getStoredNote(noteId) {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const transaction = db.transaction(['stored_notes'], 'readonly');
            const store = transaction.objectStore('stored_notes');
            
            const noteIdBase64 = uint8ArrayToBase64(noteId);
            const request = store.get(noteIdBase64);
            
            request.onsuccess = () => {
                if (request.result) {
                    // Convert base64 back to Uint8Array
                    const result = {
                        header: base64ToUint8Array(request.result.header),
                        details: base64ToUint8Array(request.result.details),
                        createdAt: request.result.createdAt
                    };
                    resolve(result);
                } else {
                    resolve(undefined);
                }
            };
            request.onerror = () => reject(new Error('Failed to get stored note'));
        } catch (error) {
            reject(error);
        }
    });
}

// Get all stored notes for a tag
export function getStoredNotesForTag(tag) {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const transaction = db.transaction(['stored_notes'], 'readonly');
            const store = transaction.objectStore('stored_notes');
            const index = store.index('tag');
            
            const request = index.getAll(tag);
            request.onsuccess = () => {
                const results = request.result.map(note => ({
                    header: base64ToUint8Array(note.header),
                    details: base64ToUint8Array(note.details),
                    createdAt: note.createdAt
                }));
                resolve(results);
            };
            request.onerror = () => reject(new Error('Failed to get stored notes for tag'));
        } catch (error) {
            reject(error);
        }
    });
}

// Record that a note has been fetched
export function recordFetchedNote(noteId, tag, fetchedAt) {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const transaction = db.transaction(['fetched_notes'], 'readwrite');
            const store = transaction.objectStore('fetched_notes');
            
            const noteIdBase64 = uint8ArrayToBase64(noteId);
            const fetchedData = {
                noteId: noteIdBase64,
                tag: tag,
                fetchedAt: fetchedAt
            };
            
            const request = store.put(fetchedData);
            request.onsuccess = () => resolve(undefined);
            request.onerror = () => reject(new Error('Failed to record fetched note'));
        } catch (error) {
            reject(error);
        }
    });
}

// Check if a note has been fetched
export function noteFetched(noteId) {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const transaction = db.transaction(['fetched_notes'], 'readonly');
            const store = transaction.objectStore('fetched_notes');
            
            const noteIdBase64 = uint8ArrayToBase64(noteId);
            const request = store.get(noteIdBase64);
            
            request.onsuccess = () => {
                resolve(request.result !== undefined);
            };
            request.onerror = () => reject(new Error('Failed to check if note fetched'));
        } catch (error) {
            reject(error);
        }
    });
}

// Get all fetched note IDs for a tag
export function getFetchedNotesForTag(tag) {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const transaction = db.transaction(['fetched_notes'], 'readonly');
            const store = transaction.objectStore('fetched_notes');
            const index = store.index('tag');
            
            const request = index.getAll(tag);
            request.onsuccess = () => {
                const results = request.result.map(note => ({
                    noteId: base64ToUint8Array(note.noteId),
                    tag: note.tag,
                    fetchedAt: note.fetchedAt
                }));
                resolve(results);
            };
            request.onerror = () => reject(new Error('Failed to get fetched notes for tag'));
        } catch (error) {
            reject(error);
        }
    });
}
