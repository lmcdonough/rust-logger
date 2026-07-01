use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// Inner implementation - idental logic to M5
// kept private - callers only see ConcurrentRateLimiter
struct RateLimiterInner {
    capacity: u32,
    window_secs: u64,
    timestamps: VecDeque<u64>,
}

impl RateLimiterInner {
    fn new(capacity: u32, window_secs: u64) -> Self {
        Self {
            capacity,
            window_secs,
            timestamps: VecDeque::with_capacity(capacity as usize),
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    fn evict_expired(&mut self, now: u64) {
        let window_start = now.saturating_sub(self.window_secs);
        while let Some(&t) = self.timestamps.front() {
            if t <= window_start { self.timestamps.pop_front(); }
            else { break; }
        }
    }

    // Returns true if allowed, false if rate limited
    // Always mutates - records timesamp on allow
    fn check(&mut self) -> bool {
        let now = Self::now_secs();
        self.evict_expired(now);

        if self.timestamps.len() as u32 >= self.capacity {
            return false; // rate limited
        }

        self.timestamps.push_back(now);
        true
    }

    fn remaining(&mut self) -> u32 {
        let now = Self::now_secs();
        self.evict_expired(now);
        self.capacity.saturating_sub(self.timestamps.len() as u32)
    }
}

// Public wrapper - Arc<Mutex<Inner>> pattern
// Clone is cheap - just increments Arc counter
// Each clone shares the same underlying rate limiter state
#[derive(Clone)]
pub struct ConcurrentRateLimiter {
    // Mutex not RwLock: check() always writes (records timestamp)
    // so there's no read only path to benefit from RwLock
    inner: Arc<Mutex<RateLimiterInner>>,
}

impl ConcurrentRateLimiter {
    pub fn new(capacity: u32, window_secs: u64) -> Self {
        assert!(capacity > 0);
        assert!(window_secs > 0);
        Self {
            inner: Arc::new(Mutex::new(RateLimiterInner::new(capacity, window_secs))),
        }
    }

    // allow() - acquires lock, checks and records, releases lock
    // The entire check and record is automatic from other threads perspective
    // This is critical: check without record would create a TOCTOU race
    pub fn allow(&self) -> bool {
        self.inner.lock().unwrap().check()
    }

    pub fn remaining(&self) -> u32 {
        self.inner.lock().unwrap().remaining()
    }

    pub fn reset(&self) {
        self.inner.lock().unwrap().timestamps.clear();
    }
}