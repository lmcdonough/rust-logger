use std::collections::HashMap;
// SystemTime: the wall clock. Duration. a span of time
// UNIX_EPOCH: the reference point (jan 1 1970 UTC)
use std::time::{SystemTime, Duration, UNIX_EPOCH};

// Our enum - two variants, each carrying exactly the data they need
// #[derive(Debug)] so we can print it in tests with {:?}
#[derive(Debug)]
enum StoreEntry {
    // Value with an expiry - stores the Unix timestamp of expiry
    WithTTL {
        value: String,
        expires_at: u64,    // seconds since UNIX_EPOCH
    },
    // Value that never expires
    Permanent {
        value: String,
    },
}

impl StoreEntry {
    // Does this entry expire before 'now'?
    // &self - read only borrow. We match on a reference to self
    // 'now' is a u64 Unix timestamp passed in (makes testing easy, no real clock needed)
    fn is_expired(&self, now: u64) -> bool {
        match self {
            // Destructure the WithTTL variant - bind expires_at 
            // '..' ignores other fields we don't need (value here)
            StoreEntry::WithTTL { expires_at, .. } => now > *expires_at,
            // Permanent entries never expire
            StoreEntry::Permanent { .. } => false,
        }
    }

    // Extract the value string as a &str - borrow from self.
    // Return type &str borrows from self, so lifetime is tied to self
    fn value(&self) -> &str {
        match self {
            // Both arms return a &str borrowed from the String inside
            StoreEntry::WithTTL { value, .. } => value,
            StoreEntry::Permanent { value } => value,
        }
    }
}

// The KV store itself - wraps a HashMap of key -> entry
pub struct KvStore {
    // HashMap<String, StoreEntry>: owned key -> owned entry
    // KvStore owns both the keys and the entries
    data: HashMap<String, StoreEntry>,
}

impl KvStore {
    pub fn new() -> Self {
        Self { data: HashMap::new() }
    }

    // Helper: get current Unix timestamp as u64
    // Returns Result - SystemTime::now() can theoretically fail on exotic platforms
    // .unwrap() is acceptable here for simplicity; production code would propogate
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()   // safe: current time is always after epoch
            .as_secs()  // convert Duration -> u64 seconds
    }

    // Set a key with a TTL (in seconds from now).
    // ttl_secs: Option<u64> - None means permanent, Some(n) means expires in n seconds
    // key: String - we take ownership (will store it as a HashMap key)
    // value: String - we take ownership (will store it in the entry)
    pub fn set(&mut self, key: String, value: String, ttl_secs: Option<u64>) {
        // Match on the TTL option to build the right variant
        let entry = match ttl_secs {
            Some(ttl) => {
                // Compute absolute expiry: now + ttl
                let expires_at = Self::now_secs() + ttl;
                StoreEntry::WithTTL { value, expires_at }
            }
            None => {
                // No TTL - permanent entry
                StoreEntry::Permanent { value }
            }
        };

        // .insert() adds or replaces the key. Old entry is dropped automatically.
        self.data.insert(key, entry);
    }

    // Get a value by key.
    // Returns Option<&str>:
    //  None - key missing or expired
    //  Some - key exists and is still valid
    // &str borrows from the HashMap, so return lifetime is tied to &self
    pub fn get(&self, key: &str) -> Option<&str> {
        // .get(key) returns Option<&StoreEntry>
        // \? propogates None if key doesn't exist - early return
        let entry = self.data.get(key)?;

        // Check expiry - pass current time in
        if entry.is_expired(Self::now_secs()) {
            // Key exists but has expired - treat as missing
            return None;
        }

        // Key exists and is valid - return borrowed &str
        Some(entry.value())
    }

    // Check if a key exists AND is not expired
    pub fn contains_key(&self, key: &str) -> bool {
        // self.get() already handles expiry check - reuse it
        self.get(key).is_some()
    }
}

impl KvStore {
    // Remove a key unconditionally
    // Returns true if the key existed (even if expired), false if it was missing
    pub fn delete(&mut self, key: &str) -> bool {
        // HashMap::remove returns Option<V> - Some(entry) if removed, None if absents
        // .is_some() converts that to a bool
        self.data.remove(key).is_some()
    }

    // Remove all expired entries. Returns count of entries removed
    // &mut self - we're modifying the HashMap
    pub fn purge_expired(&mut self) -> usize {
        let now = Self::now_secs();

        // Collect keys to remove first - can't mutate data while iterating it
        // (borrow checker: iterating borrows self.data, remove() mutably borrows it)
        // Python equivalent: [k for k, v in d.items() if v.is_expired()]
        let expired_keys: Vec<String> = self
            .data
            .iter() // iterate(key, value) pairs as (&String, &StoreEntry)
            // filter: keep only pairs where entry IS expired
            .filter(|(_, entry)| entry.is_expired(now))
            // extract just the key, clone it (we need owned Strings for remove())
            .map(|(key, _)| key.clone())
            .collect();

        let count = expired_keys.len();

        // Now remove them - data is no longer borrowed by the iterator
        for key in expired_keys {
            self.data.remove(&key);
        }

        count
    }

    // How many keys are stored (including expired ones not yet purged)
    pub fn len(&self) -> usize {
        self.data.len()
    }

    // How many keys are currently valid (not expired)?
    pub fn active_len(&self) -> usize {
        let now = Self::now_secs();
        self.data
            .values()                       // iterate &StoreEntry values
            .filter(|e| !e.is_expired(now)) // keep non expired
            .count()                        // count them
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_set_and_get_permanent() {
        let mut store = KvStore::new();
        store.set("name".to_string(), "alice".to_string(), None);
        assert_eq!(store.get("name"), Some("alice"));
        assert_eq!(store.get("missing"), None);
    }

    #[test]
    fn test_ttl_expiry() {
        let mut store = KvStore::new();
        // TTL of 1 second
        store.set("token".to_string(), "abc123".to_string(), Some(1));

        // Immediately after set - should exist
        assert_eq!(store.get("token"), Some("abc123"));

        // Wait 2 seconds for it to expire
        thread::sleep(Duration::from_secs(2));

        // After TTL - should be None
        assert_eq!(store.get("token"), None);
    }

    #[test]
    fn test_delete() {
        let mut store = KvStore::new();
        store.set("key".to_string(), "val".to_string(), None);
        assert!(store.delete("key"));       // existed, returns true
        assert!(!store.delete("key"));      // already gone, returns false
        assert_eq!(store.get("key"), None);
    }

    #[test]
    fn test_purge_expired() {
        let mut store = KvStore::new();
        store.set("expires".to_string(), "soon".to_string(), Some(1));
        store.set("permanent".to_string(), "forever".to_string(), None);

        thread::sleep(Duration::from_secs(2));

        let removed = store.purge_expired();
        assert_eq!(removed, 1);
        assert_eq!(store.len(), 1);     // only "permanent" remains
        assert_eq!(store.get("permanent"), Some("forever"));
    }

    #[test]
    fn test_active_len() {
        let mut store = KvStore::new();
        store.set("a".to_string(), "1".to_string(), Some(1));
        store.set("b".to_string(), "2".to_string(), None);
        store.set("c".to_string(), "3".to_string(), None);

        assert_eq!(store.active_len(), 3);

        thread::sleep(Duration::from_secs(2));

        // "a" expired - only b and c active
        assert_eq!(store.active_len(), 2);
    }
}