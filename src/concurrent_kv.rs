use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

// Same entry enum as M3 - unchanged
// Derived Clone so we can return owned values from get()
#[derive(Debug, Clone)]
enum StoreEntry {
    WithTTL { value: String, expires_at: u64 },
    Permanent { value: String },
}

impl StoreEntry {
    fn is_expired(&self, now: u64) -> bool {
        match self {
            StoreEntry::WithTTL { expires_at, .. } => now > *expires_at,
            StoreEntry::Permanent { .. } => false,
        }
    }

    fn value(&self) -> &str {
        match self {
            StoreEntry::WithTTL { value, .. } => value,
            StoreEntry::Permanent { value } => value,
        }
    }
}

// ConcurrentKVStore: Arc<RwLock<HashMap>> pattern
// Clone is cheap - just increments the Arc counter
// The actual HashMap lives once on the heap
#[derive(Clone)]
pub struct ConcurrentKvStore {
    // RwLock because:
    // - get() is read only (after expiry check - expiry doesn't mutate)
    // - set()/delete()/purge() are writes
    // - Read heavy workloads benefit from concurrent reads
    inner: Arc<RwLock<HashMap<String, StoreEntry>>>,
}

impl ConcurrentKvStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    // get() acquires a READ lock - multiple threads can call this simultaneously
    // Returns Option<String> - cloned so the lock can be released immediately
    // We clone the value rather than returning a reference because the
    // RwLock would need to stay locked fo the reference's lifetime
    pub fn get(&self, key: &str) -> Option<String> {
        // .read() blocks if a write lock is held, then acquires shared read lock
        let store = self.inner.read().unwrap();
        let entry = store.get(key)?;

        if entry.is_expired(Self::now_secs()) {
            return None;
        }

        // .to_string() clones the &str into an owned String
        // Lock releases when 'store' drops at end of function
        Some(entry.value().to_string())
    }

    // set acquires a WRITE lock - exclusive, all readers/writers block
    pub fn set(&self, key: String, value: String, ttl_secs: Option<u64>) {
        // .write() blocks until all read locks AND write locks are released
        let mut store = self.inner.write().unwrap();

        let entry = match ttl_secs {
            Some(ttl) => StoreEntry::WithTTL {
                value,
                expires_at: Self::now_secs() + ttl,
            },
            None => StoreEntry::Permanent { value },
        };
        
        store.insert(key, entry);
        // Write lock releases here - MutexGuard drops
    }

    // delete() - write lock, exclusive
    pub fn delete(&self, key: &str) -> bool {
        self.inner.write().unwrap().remove(key).is_some()
    }

    // purge_expired() - write lock, iterates and removes expired entries
    // Same collect first pattern as M3 - can't remove while iterating
    pub fn purge_expired(&self) -> usize {
        let now = Self::now_secs();
        let mut store = self.inner.write().unwrap();

        let expired: Vec<String> = store
            .iter()
            .filter(|(_, e)| e.is_expired(now))
            .map(|(k, _)| k.clone())
            .collect();

        let count = expired.len();
        for k in expired {
            store.remove(&k);
        }
        count
    }

    // active_len() - read lock sufficient, just counting
    pub fn active_len(&self) -> usize {
        let now = Self::now_secs();
        self.inner.read().unwrap()
            .values()
            .filter(|e| !e.is_expired(now))
            .count()
    }

}

