// thiserror: add to Cargo.toml: thiserror = "1"
use thiserror::Error;
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

// Our custom error type
// thiserror derives Display and std::error::Error automatically
// The #[error("...")] attribute sets what println!("{}", e) shows
#[derive(Debug, Error, PartialEq)]
pub enum RateLimitError {
    // Caller hit the limit - carries how many requests were in the window
    #[error("rate limit exceeded: {requests} requests in {window_secs}s window")]
    LimitExceeded {
        requests: u32,
        window_secs: u64,
    },

    // Misconfiguration caught at construction time
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

// The rate limiter itself
// Own all its data - no lifetime annotations needed
// Debug: lets tests use .unwrap_err() on Result<RateLimiter, _>
#[derive(Debug)]
pub struct RateLimiter {
    // Max requests allowed within the window
    capacity: u32,

    // Window duration in seconds
    window_secs: u64,

    // Timestamps of requests within the current window
    // VecDeque: O(1) pop_front (evict_old) + O(1) push_back (record_new)
    // Stores Unix timestamps as u64 - matches what SystemTime gives us
    timestamps: VecDeque<u64>,
}

impl RateLimiter {
    // Constructor - validates config and returns Result
    // Returning Result<Self> instead of Self lets us reject bad configs
    // at build time with a typed error rather than panicking
    pub fn new(capacity: u32, window_secs: u64) -> Result<Self, RateLimitError> {
        // Validate - zero capacity or zero window makes no sense
        if capacity == 0 {
            return Err(RateLimitError::InvalidConfig(
                "capacity must be > 0".to_string()
            ));
        }
        if window_secs == 0 {
            return Err(RateLimitError::InvalidConfig(
                "window_secs must be > 0".to_string()
            ));
        }

        Ok(Self {
            capacity,
            window_secs,
            // pre-allocate capacity slots - avoids realloc on first N pushes
            timestamps: VecDeque::with_capacity(capacity as usize),
        })
    }

    // Get current Unix timestamp in seconds
    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    // Evict timestamps that have fallen outside the window
    // Called before every allow() check.
    //
    // Why &mut self: we're modifying timestamps (popping from the front)
    // Why seperate method: single responsibility - check() calls this first
    fn evict_expired(&mut self, now: u64) {
        // The window starts at (now - window_secs)
        // Any timestamp before that is expired
        let window_start = now.saturating_sub(self.window_secs);
        // saturating_sub: u64 subtraction that floors at 0 instead of
        // wrapping around (underflowing). Safe when now < window_secs

        // Pop from the front while the oldest timestamp is outside the window
        // VecDeque::front() peeks without removing - returns Option<&u64>
        // while let Some(&ts) = ... : destructure the Option<u64> each loop.
        while let Some(&ts) = self.timestamps.front() {
            if ts <= window_start {
                self.timestamps.pop_front(); // O(1) - evict oldest
            } else {
                break; // front is still in window - rest are too (sorted)
            }
        }
    }

    // Core method: should this request be allowed?
    // Returns Ok(()) if allowed, Err(LimitExceeded) if over limit
    // &mut self because we may record the timestamp (modifies self)
    pub fn check(&mut self) -> Result<(), RateLimitError> {
        let now = Self::now_secs();

        // Step 1: remove timestamps outside the current window
        self.evict_expired(now);

        // Step 2: count how many requests are in the window now
        let current = self.timestamps.len() as u32;

        // Step 3: if at or over capacity, reject
        if current >= self.capacity {
            return Err(RateLimitError::LimitExceeded {
                requests: current,
                window_secs: self.window_secs
            });
        } 

        // Step 4: under limit - record this request's timestamp and allow
        self.timestamps.push_back(now);
        Ok(())
        
    }
}

impl RateLimiter {
    // allow() is a friendlier alias - returns bool instead of Result
    // Some callers prefer if limiter.allow() { ... } over match
    pub fn allow(&mut self) -> bool {
        self.check().is_ok()
    }

    // How many requests are currently recorded in the window?
    // Evicts expired first so the count is accurate
    pub fn current_count(&mut self) -> u32 {
        let now = Self::now_secs();
        self.evict_expired(now);
        self.timestamps.len() as u32
    }

    // How many more requests are allowed before hitting the limit?
    pub fn remaining(&mut self) -> u32 {
        // saturating_sub: floors at 0 - never returns a "negative" u32
        self.capacity.saturating_sub(self.current_count())
    }

    // Reset the limiter - clear all recorded timestamps
    // Useful after a burst: wipe the window and start fresh.
    pub fn reset(&mut self) {
        self.timestamps.clear();
    }

    // Read-only config accessors - &self, no mutation
    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    
    pub fn window_secs(&self) -> u64 {
        self.window_secs
    }

    // Is the limiter currently at capacity?
    pub fn is_throttled(&mut self) -> bool {
        self.remaining() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_allows_within_limit() {
        // capacity=3, window=60s - show allow first 3 requests
        let mut r1 = RateLimiter::new(3, 60).unwrap();

        assert_eq!(r1.check(), Ok(()));
        assert_eq!(r1.check(), Ok(()));
        assert_eq!(r1.check(), Ok(()));
    }

    #[test]
    fn test_rejects_over_limit() {
        let mut r1 = RateLimiter::new(2, 60).unwrap();

        assert!(r1.check().is_ok());
        assert!(r1.check().is_ok());

        // Third request should be rejected
        let result = r1.check();
        assert!(result.is_err());

        // Error carries the right data
        match result.unwrap_err() {
            RateLimitError::LimitExceeded { requests, window_secs } => {
                assert_eq!(requests, 2);
                assert_eq!(window_secs, 60);
            }
            _ => panic!("wrong error variant"),
        }
    }

    #[test]
    fn test_window_slides_correctly() {
        // capacity=2, window=1, second - tight window for fast test
        let mut r1 = RateLimiter::new(2, 1).unwrap();
    

        assert!(r1.check().is_ok());    // request 1
        assert!(r1.check().is_ok());    // request 2
        assert!(r1.check().is_err());   // request 3 - rejected

        // Wait for the window to expire
        thread::sleep(Duration::from_millis(1100));

        // Old requests evicted - window fresh
        assert!(r1.check().is_ok());    // allowed again
        assert!(r1.check().is_ok());
    }

    #[test]
    fn test_remaining_and_reset() {
        let mut rl = RateLimiter::new(5, 60).unwrap();

        rl.check().unwrap();
        rl.check().unwrap();

        assert_eq!(rl.remaining(), 3);
        assert_eq!(rl.current_count(), 2);

        rl.reset();

        assert_eq!(rl.remaining(), 5);
        assert_eq!(rl.current_count(), 0);
    }

    #[test]
    fn test_invalid_config() {
        // capacity=0 should return Err at construction
        let result = RateLimiter::new(0, 60);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RateLimitError::InvalidConfig(_)
        ));
    }

    #[test]
    fn test_allow_bool_api() {
        let mut rl = RateLimiter::new(1, 60).unwrap();

        assert!(rl.allow());   // first — allowed
        assert!(!rl.allow());  // second — blocked
    }
}