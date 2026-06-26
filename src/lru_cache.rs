use std::collections::HashMap;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

// LruCache: two synchronized data structures
// The HashMap gives (1) lookup by key
// The BTreeMap gives O(log n) access to the oldest entry by timestamp
// Together they give us O(log n) get and put - interview acceptable
pub struct LruCache {
    // Maximum number of entries before eviction kicks in
    capacity: usize,

    // key -> (value, timestamp): what we store and when it was last used
    // String keys and values keep the interface simple for interviews
    store: HashMap<String, (String, u64)>,

    // timestamp -> key: ordered map lets us find the OLDEST entry in O(log n)
    // BTreeMap is always sorted by key (timestamp here) - iter().next() = oldest
    order: BTreeMap<u64, String>,

    // Monotonically increasing counter - each access gets a unique timestamp
    // Using a counter instead of SystemTime avoids platform complexity in tests
    counter: u64,
}

impl LruCache {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "LRU capacity must be > 0");
        Self {
            capacity,
            store: HashMap::new(),
            order: BTreeMap::new(),
            counter: 0,
        }
    }

    // Increment and return the next unique timestamp
    fn next_ts(&mut self) -> u64 {
        self.counter += 1;
        self.counter
    }

    // get() - look up a key, marking it as most recently used.
    // Returns Option<String> - None if missing
    // &mut self because we update the timestamp on access (LRU requires this)
    pub fn get(&mut self, key: &str) -> Option<String> {
        // .get() on HashMap returns Option<&(String, u64)>
        // ? propagates None if key doesn't exist; clone gives us owned
        // copies of the value and old timestamp so we can mutate store after
        let (value, old_ts) = self.store.get(key)?.clone();

        // Remove the old recency entry - this key is no longer at old_ts
        self.order.remove(&old_ts);

        // Assign a fresh timestamp - moves this key to "most recently used"
        let new_ts = self.next_ts();
        self.order.insert(new_ts, key.to_string());

        // Update store with new timestamp, keep same value
        self.store.insert(key.to_string(), (value.clone(), new_ts));

        Some(value)
    }

    // put() - insert or update a key-value pair
    // Evicts the LRU entry if we're at capacity
    // &mut self - we're always modifying both structures
    pub fn put(&mut self, key: String, value: String) {
        // Case 1: key already exists - update value and refresh timestamp
        if let Some((_, old_ts)) = self.store.get(&key).cloned() {
            // Remove the old ordering entry for this key
            self.order.remove(&old_ts);

            // Insert fresh timestamp into order
            let ts = self.next_ts();
            self.order.insert(ts, key.clone());

            // Update store: same key, new value, new timestamp
            self.store.insert(key, (value, ts));
            return;
        }

        // Case 2: new key, but at capacity - evict LRU entry first
        if self.store.len() == self.capacity {
            // BTreeMap::iter() yields entries in KEY order ( ascending timestamp)
            // .next() gives the SMALLEST timestamp = OLDEST entry
            // .cloned() because we need owned values to call remove()
            if let Some((&oldest_ts, oldest_key)) = self.order.iter().next() {
                let oldest_key = oldest_key.clone(); // clone before mutable borrow
                self.order.remove(&oldest_ts);
                self.store.remove(&oldest_key);
            }
        }

        // Case 3: insert new key
        let ts = self.next_ts();
        self.order.insert(ts, key.clone());
        self.store.insert(key, (value, ts));
        
    }

    // How many entries are currently stored?
    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

// ThreadSafeLruCache wraps LruCache in Arc<Mutex<T>>
// Arc: multiple threads can hold a reference to it
// Mutex: only one thread accesses it at a time
//
// Clone is cheap - it just increments the Arc counter
// The actual cache data lives once on the heap
#[derive(Clone)]
pub struct ThreadSafeLruCache {
    inner: Arc<Mutex<LruCache>>,
}

impl ThreadSafeLruCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    // get() acquires the lock, delegates to inner, releases lock automatically
    // Returns Option<String> - same interface as single threaded version
    pub fn get(&self, key: &str) -> Option<String> {
        // .lock() returns Result<MutexGuard<LRUCache>, PoisonError>
        // .unwrap() panics if a thread panicked while holding the lock (rare)
        // MutexGuard<LruCache> derefs to &LruCache - same methods
        self.inner.lock().unwrap().get(key)
    }

    pub fn put(&self, key: String, value: String) {
        // Lock acquired -> put() called -> MutexGuard dropped -> lock released
        // The entire put() is atomic from the other threads' perspective
        self.inner.lock().unwrap().put(key, value);
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_basic_put_and_get() {
        let mut cache = LruCache::new(2);

        cache.put("a".to_string(), "1".to_string());
        cache.put("b".to_string(), "2".to_string());

        assert_eq!(cache.get("a"), Some("1".to_string()));
        assert_eq!(cache.get("b"), Some("2".to_string()));
        assert_eq!(cache.get("c"), None);
    }

    #[test]
    fn test_eviction_evicts_lru() {
        let mut cache = LruCache::new(2);

        cache.put("a".to_string(), "1".to_string());
        cache.put("b".to_string(), "2".to_string());

        // Access "a" makes it MRU "b" becomes LRU
        cache.get("a");

        // Insert "c" - capcity exceeded, "b" (LRU) should be evicted
        cache.put("c".to_string(), "3".to_string());

        assert_eq!(cache.get("a"), Some("1".to_string())); // still here
        assert_eq!(cache.get("b"), None);                  // evicted
        assert_eq!(cache.get("c"), Some("3".to_string()))  // new entry
    }

    #[test]
    fn test_update_existing_key() {
        let mut cache = LruCache::new(2);

        cache.put("a".to_string(), "1".to_string());
        cache.put("b".to_string(), "2".to_string());
        cache.put("a".to_string(), "updated".to_string()); // update "a"

        // "a" updated and is now MRU — "b" is LRU
        cache.put("c".to_string(), "3".to_string()); // evicts "b"

        assert_eq!(cache.get("a"), Some("updated".to_string()));
        assert_eq!(cache.get("b"), None);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_thread_safe_concurrent_writes() {
        let cache = ThreadSafeLruCache::new(100);
        let mut handles = vec![];

        // Spawn 10 threads, each inserting 10 entries
        for i in 0..10 {
            let c = cache.clone(); // cheap Arc clone
            handles.push(thread::spawn(move || {
                for j in 0..10 {
                    c.put(
                        format!("key-{}-{}", i, j),
                        format!("val-{}-{}", i, j)
                    )
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // All 100 entries should be present (capcity = 100)
        assert_eq!(cache.len(), 100);
    }  
    
    #[test]
    fn test_thread_safe_read_after_write() {
        let cache = ThreadSafeLruCache::new(10);

        cache.put("name".to_string(), "alice".to_string());

        let c = cache.clone();
        let handle = thread::spawn(move || {
            // Read from a different thread
            assert_eq!(c.get("name"), Some("alice".to_string()));
        });

        handle.join().unwrap();
    }    
}