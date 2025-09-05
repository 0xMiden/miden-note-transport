// Management IndexedDB Operations
// This module handles statistics and maintenance operations

// Helper function to get database
async function getDatabase() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open('miden_transport_client', 1);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(new Error('Failed to open database'));
    });
}

// Get database statistics
export function getStats() {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            
            // Count stored notes
            const storedNotesTransaction = db.transaction(['stored_notes'], 'readonly');
            const storedNotesStore = storedNotesTransaction.objectStore('stored_notes');
            const storedNotesCount = await new Promise((resolve, reject) => {
                const request = storedNotesStore.count();
                request.onsuccess = () => resolve(request.result);
                request.onerror = () => reject(new Error('Failed to count stored notes'));
            });
            
            // Count fetched notes
            const fetchedNotesTransaction = db.transaction(['fetched_notes'], 'readonly');
            const fetchedNotesStore = fetchedNotesTransaction.objectStore('fetched_notes');
            const fetchedNotesCount = await new Promise((resolve, reject) => {
                const request = fetchedNotesStore.count();
                request.onsuccess = () => resolve(request.result);
                request.onerror = () => reject(new Error('Failed to count fetched notes'));
            });
            
            // Count unique tags
            const tagMappingsTransaction = db.transaction(['tag_mappings'], 'readonly');
            const tagMappingsStore = tagMappingsTransaction.objectStore('tag_mappings');
            const uniqueTagsCount = await new Promise((resolve, reject) => {
                const request = tagMappingsStore.count();
                request.onsuccess = () => resolve(request.result);
                request.onerror = () => reject(new Error('Failed to count unique tags'));
            });
            
            const stats = {
                fetchedNotesCount: fetchedNotesCount,
                storedNotesCount: storedNotesCount,
                uniqueTagsCount: uniqueTagsCount
            };
            
            resolve(stats);
        } catch (error) {
            reject(error);
        }
    });
}

// Clean up old data based on retention policy
export function cleanupOldData(retentionDays) {
    return new Promise(async (resolve, reject) => {
        try {
            const db = await getDatabase();
            const cutoffDate = new Date();
            cutoffDate.setDate(cutoffDate.getDate() - retentionDays);
            const cutoffDateString = cutoffDate.toISOString();
            
            let cleanedCount = 0;
            
            // Clean up old stored notes
            const storedNotesTransaction = db.transaction(['stored_notes'], 'readwrite');
            const storedNotesStore = storedNotesTransaction.objectStore('stored_notes');
            const storedNotesIndex = storedNotesStore.index('createdAt');
            
            const storedNotesRequest = storedNotesIndex.openCursor();
            storedNotesRequest.onsuccess = (event) => {
                const cursor = event.target.result;
                if (cursor) {
                    if (cursor.value.createdAt < cutoffDateString) {
                        cursor.delete();
                        cleanedCount++;
                    }
                    cursor.continue();
                } else {
                    // Clean up old fetched notes
                    const fetchedNotesTransaction = db.transaction(['fetched_notes'], 'readwrite');
                    const fetchedNotesStore = fetchedNotesTransaction.objectStore('fetched_notes');
                    const fetchedNotesIndex = fetchedNotesStore.index('fetchedAt');
                    
                    const fetchedNotesRequest = fetchedNotesIndex.openCursor();
                    fetchedNotesRequest.onsuccess = (event) => {
                        const cursor = event.target.result;
                        if (cursor) {
                            if (cursor.value.fetchedAt < cutoffDateString) {
                                cursor.delete();
                                cleanedCount++;
                            }
                            cursor.continue();
                        } else {
                            resolve(cleanedCount);
                        }
                    };
                    fetchedNotesRequest.onerror = () => reject(new Error('Failed to clean up fetched notes'));
                }
            };
            storedNotesRequest.onerror = () => reject(new Error('Failed to clean up stored notes'));
            
        } catch (error) {
            reject(error);
        }
    });
}
