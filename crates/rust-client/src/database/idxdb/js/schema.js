// IndexedDB Schema and Database Initialization

export function openDatabase() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open('miden_transport_client', 2);
        
        request.onerror = () => {
            reject(new Error('Failed to open IndexedDB database'));
        };
        
        request.onsuccess = () => {
            resolve(request.result);
        };
        
        request.onupgradeneeded = (event) => {
            const db = event.target.result;
            
            // Create object stores
            if (!db.objectStoreNames.contains('stored_notes')) {
                const storedNotesStore = db.createObjectStore('stored_notes', { keyPath: 'noteId' });
                storedNotesStore.createIndex('tag', 'tag', { unique: false });
                storedNotesStore.createIndex('createdAt', 'createdAt', { unique: false });
            }
            
            if (!db.objectStoreNames.contains('fetched_notes')) {
                const fetchedNotesStore = db.createObjectStore('fetched_notes', { keyPath: 'noteId' });
                fetchedNotesStore.createIndex('tag', 'tag', { unique: false });
                fetchedNotesStore.createIndex('fetchedAt', 'fetchedAt', { unique: false });
            }
        };
    });
}
