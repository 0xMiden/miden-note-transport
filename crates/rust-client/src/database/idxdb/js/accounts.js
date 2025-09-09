// Account IndexedDB Operations
// This module handles all account-related database operations

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
        const request = indexedDB.open('miden_transport_client', 1);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(new Error('Failed to open database'));
    });
}

// Store a tag to account ID mapping
export function storeTagAccountMapping(tag, accountId) {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const transaction = db.transaction(['tag_mappings'], 'readwrite');
            const store = transaction.objectStore('tag_mappings');
            
            const mappingData = {
                tag: tag,
                accountId: uint8ArrayToBase64(accountId)
            };
            
            const request = store.put(mappingData);
            request.onsuccess = () => resolve(undefined);
            request.onerror = () => reject(new Error('Failed to store tag account mapping'));
        } catch (error) {
            reject(error);
        }
    });
}

// Get all tag to account ID mappings
export function getAllTagAccountMappings() {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const transaction = db.transaction(['tag_mappings'], 'readonly');
            const store = transaction.objectStore('tag_mappings');
            
            const request = store.getAll();
            request.onsuccess = () => {
                const results = request.result.map(mapping => ({
                    tag: mapping.tag,
                    accountId: base64ToUint8Array(mapping.accountId)
                }));
                resolve(results);
            };
            request.onerror = () => reject(new Error('Failed to get tag account mappings'));
        } catch (error) {
            reject(error);
        }
    });
}
