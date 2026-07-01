use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

// A log line as it travels through the channel
// Must be Send - String is Send, so LogLine is Send
#[derive(Debug, Clone)]
pub struct LogLine {
    pub source: String, // which service sent this
    pub content: String, // the raw log string
}

// LogPipeline: owns the sender end and consumer thread handle
pub struct LogPipeline {
    // Sender is cloneable - give one to each producer thread
    // Wrapped in Arc<mutex> so multiple handles can clone it safely
    tx: Arc<Mutex<mpsc::Sender<LogLine>>>,

    // JoinHandle for the consumer thread
    // Option so we can take() it during shutdown (drop + join)
    consumer: Option<thread::JoinHandle<Vec<LogLine>>>,
}

impl LogPipeline {
    // Spawn the consumer thread and return the pipeline handle
    pub fn new() -> Self {
        // channel() creates a linked(tx, rx) pair
        // tx: Sender<LogLine> - single consumer, moved into the thread
        let (tx, rx) = mpsc::channel::<LogLine>();

        // Spawn the consumer threads - owns rx
        // move closure: rx is moved into the thread (satisfies the `static bound)
        let consumer = thread::spawn(move || {
            let mut processed: Vec<LogLine> = Vec::new();

            // for msg in rx: yields each message as it arrives
            // Blocks between messages. Loop ends when ALL senders are dropped
            for line in rx {
                // In production: feed into LogParser from M2
                // Here: collect for testability
                processed.push(line);
            }

            processed // returned from thread, accessible via handle.join()
        });

        Self {
            tx: Arc::new(Mutex::new(tx)),
            consumer: Some(consumer),
        }
    }

    // Get a producer handle - clone of of the sender, safe to move into a thread
    // Each call gives a new handle pointing to the same channel
    pub fn producer(&self) -> mpsc::Sender<LogLine> {
        // Clone the Sender - increments the internal channel reference count
        // When all senders are dropped, the Receiver's for loop ends
        self.tx.lock().unwrap().clone()
    }

    // Shutdown: drop the original sender, wait for consumer to finish
    // Returns all processed log lines from the consumer thread
    pub fn shutdown(mut self) -> Vec<LogLine> {
        // Drop our sender - when combined with producers dropping theirs
        // the channel closes and the consumer's for loop exits
        drop(self.tx);

        // join() waits for the consumer thread to finish
        // Returns Result<Vec<LogLine>, JoinError>
        // take() removes the handle from the Option so we can call join()
        self.consumer.take().unwrap().join().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concurrent_kv::ConcurrentKvStore;
    use crate::concurrent_rate_limiter::ConcurrentRateLimiter;
    use std::thread;

    // --- ConcurrentKvStore ---

    #[test]
    fn test_concurrent_kv_reads() {
        let store = ConcurrentKvStore::new();
        store.set("k".to_string(), "v".to_string(), None);

        // Spawn 10 threads all reading simultaneously
        let mut handles = vec![];
        for _ in 0..10 {
            let s = store.clone(); // cheap Arc clone
            handles.push(thread::spawn(move || {
                assert_eq!(s.get("k"), Some("v".to_string()));
            }));
        }
        for h in handles { h.join().unwrap(); }
    }

    #[test]
    fn test_concurrent_kv_writes() {
        let store = ConcurrentKvStore::new();
        let mut handles = vec![];

        // 10 threads each writing a different key
        for i in 0..10 {
            let s = store.clone();
            handles.push(thread::spawn(move || {
                s.set(format!("key-{}", i), format!("val-{}", i), None);
            }));
        }
        for h in handles { h.join().unwrap(); }

        assert_eq!(store.active_len(), 10);
    }

    // --- ConcurrentRateLimiter ---

    #[test]
    fn test_concurrent_rate_limiter_correct_count() {
        // capacity=5 — exactly 5 requests should be allowed across 10 threads
        let rl = ConcurrentRateLimiter::new(5, 60);
        let allowed = Arc::new(Mutex::new(0u32));
        let mut handles = vec![];

        for _ in 0..10 {
            let r = rl.clone();
            let a = Arc::clone(&allowed);
            handles.push(thread::spawn(move || {
                if r.allow() {
                    *a.lock().unwrap() += 1;
                }
            }));
        }
        for h in handles { h.join().unwrap(); }

        // Exactly 5 allowed — no TOCTOU race allowing more
        assert_eq!(*allowed.lock().unwrap(), 5);
    }

    #[test]
    fn test_rate_limiter_no_toctou() {
        // High concurrency stress test — capacity 1, 100 threads
        let rl = ConcurrentRateLimiter::new(1, 60);
        let allowed = Arc::new(Mutex::new(0u32));
        let mut handles = vec![];

        for _ in 0..100 {
            let r = rl.clone();
            let a = Arc::clone(&allowed);
            handles.push(thread::spawn(move || {
                if r.allow() { *a.lock().unwrap() += 1; }
            }));
        }
        for h in handles { h.join().unwrap(); }

        // Only 1 allowed — Mutex ensures atomic check+record
        assert_eq!(*allowed.lock().unwrap(), 1);
    }

    // --- LogPipeline ---

    #[test]
    fn test_pipeline_processes_all_messages() {
        let pipeline = LogPipeline::new();

        // Spawn 5 producer threads, each sending 3 messages
        let mut handles = vec![];
        for i in 0..5 {
            let tx = pipeline.producer();
            handles.push(thread::spawn(move || {
                for j in 0..3 {
                    tx.send(LogLine {
                        source:  format!("service-{}", i),
                        content: format!("[INFO] message {}-{}", i, j),
                    }).unwrap();
                }
                // tx drops here — producer done
            }));
        }

        for h in handles { h.join().unwrap(); }

        // Shutdown waits for consumer to drain the channel
        let processed = pipeline.shutdown();

        // All 15 messages received (5 threads × 3 messages)
        assert_eq!(processed.len(), 15);
    }

    #[test]
    fn test_pipeline_channel_closes_on_all_drops() {
        let pipeline = LogPipeline::new();
        let tx = pipeline.producer();

        tx.send(LogLine {
            source: "test".to_string(),
            content: "[ERROR] something failed".to_string(),
        }).unwrap();

        drop(tx); // producer done
        let processed = pipeline.shutdown();

        assert_eq!(processed.len(), 1);
        assert!(processed[0].content.contains("ERROR"));
    }
}