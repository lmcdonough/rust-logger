// VecDeque: double-ended queue - O(1) push_back and pop_front
// This is the key data structure choice for a sliding window
use std::collections::VecDeque;

// WindowStats: a plain data struct - no methods, just fields
// Returned by stats() so the caller can use individual values
// Clone + Debug for convenience in tests and logging
#[derive(Debug, Clone)]
pub struct WindowStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub sum: f64,
    pub count: usize,
}

// RollingWindow<T>: generic for any numeric type T.
//
// Trait bounds explained:
//  Copy -> T can be bit-copied cheaply (all primitives: i32, f64, u64, etc...)
//  Into<f64> -> T can be converted to f64 for arithmetic
//      (i32, u32, f32 all implement Into<f64>)
//
//
// We store T, not f64, to preserve the original type.
// Conversion to f64 only happens inside stats()
pub struct RollingWindow<T>
where
    T: Copy + Into<f64>,
{
    // VecDeque<T>: the window buffer. Oldest at front, newest at back
    data: VecDeque<T>,

    // Maximum number of values the window holds at once
    // When len == capacity, the next push evict the oldest
    capacity: usize,
}

impl<T> RollingWindow<T>
where
    T: Copy + Into<f64>,
{
    // Constructor - pre-allocate the VecDeque's internal buffer
    // with_capacity avoids reallocation as we fill up
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "window capacity must be > 0");
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    // Push a new value into the window
    // If the window is full, evict the oldest value first (pop_front)
    // Then push the new value to the back (push_back)
    //
    // &must self - we're modifying the VecDeque
    // value: T - we take ownership; T is Copy so the caller's copy is unaffected
    pub fn push(&mut self, value: T) {
        // If at capacity: remove the oldest item (front of the deque)
        // pop_front() is O(1) - this is why we use VecDeque, not Vec
        if self.data.len() == self.capacity {
            self.data.pop_front();
        }

        // Add new value to the back - always O(1) amortized
        // The window now contains the last 'capacity' values
        self.data.push_back(value);
    }

    // How many values are currently in the window?
    pub fn len(&self) -> usize {
        self.data.len()
    }

    // Is the window empty?
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // Is the window at full capacity?
    pub fn is_full(&self) -> bool {
        self.data.len() == self.capacity
    }

    // Return all current values oldest -> newest as Vec<T>
    // Clones each T (Copy types clone = bit copy, free)
    pub fn values(&self) -> Vec<T> {
        // .iter() gives &T, .copied() dereferences each &T to T (uses Copy)
        self.data.iter().copied().collect()
    }
}

impl<T> RollingWindow<T>
where
    T: Copy + Into<f64>,
{
    // Compute aggregate statistics over the current window
    // Returns None if the window is empty (no data to aggregate)
    // Returns Some(WindowStats) with min/max/mean/sum/count otherwise
    pub fn stats(&self) -> Option<WindowStats> {
        // Guard: nothing to compute on an empty window.
        if self.data.is_empty() {
            return None;
        }

        // Convert all T values to f64 for arithmetic
        // .iter()          -> borrows each T as &T
        // .copied()        -> dereferences &T to T (safe: T: Copy)
        // .map(Into::into) -> converts each T to f64 using the Into Trait bound
        // .collect()       -> gathers into Vec<f64> for multi-pass iteration
        let vals: Vec<f64> = self.data
            .iter()
            .copied()
            .map(Into::into) // equivalent to .map(|x| x.into())
            .collect();

        // sum: iterator adaptor - adds all f64 values
        // ::<f64> is the turbofish - tells the compiler the sum type
        let sum = vals.iter().sum::<f64>();

        let count = vals.len();

        // mean: sum divided by count, cast usize -> f64 for division
        let mean = sum / count as f64;

        // min: fold with f64::min
        // f64::INFINITY is the identity for min - any real value beats it
        // .cloned() converts &f64 to f64 so f64::min(f64, f64) works
        let min = vals
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);

        // max: fold with f64::max
        // f64::NEG_INFINITY is the identity for max - any real value beats it
        let max = vals
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        Some(WindowStats { min, max, mean, sum, count })
    }

    // Convenience: just the mean, without allocating a Vec
    // Returns None if empty, Some(mean) otherwise
    pub fn mean(&self) -> Option<f64> {
        if self.data.is_empty() {
            return None;
        }
        // Single-pass: sum all values converted to f64, divide by count
        let sum: f64 = self.data.iter().copied().map(Into::into).sum();
        Some(sum / self.data.len() as f64)
    }

    // Convenience: count of values above a threshold
    // Useful for: "how many requests in the last N exceeded 500ms?"
    pub fn count_above(&self, threshold: f64) -> usize {
        self.data
            .iter()
            .copied()
            .map(Into::into)            // T -> f64
            .filter(|&v| v > threshold) // keep values above threshold
            .count()                    // consuming: returns size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_eviction() {
        // Window of size 3
        let mut w: RollingWindow<i32> = RollingWindow::new(3);

        w.push(10);
        w.push(20);
        w.push(30);

        // Full - values are [10, 20, 30]
        assert!(w.is_full());
        assert_eq!(w.values(), vec![10, 20, 30]);

        // Push 40 - evicts 10, window becomes [20, 30, 40]
        w.push(40);
        assert_eq!(w.values(), vec![20, 30, 40]);
        assert_eq!(w.len(), 3); // still 3 - not 4
    }

    #[test]
    fn test_stats_basic() {
        let mut w: RollingWindow<f64> = RollingWindow::new(4);

        // Empty window - stats returns None
        assert!(w.stats().is_none());

        w.push(1.0);
        w.push(2.0);
        w.push(3.0);
        w.push(4.0);

        let s = w.stats().unwrap();
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 4.0);
        assert_eq!(s.mean, 2.5);
        assert_eq!(s.sum, 10.0);
        assert_eq!(s.count, 4);
    }

    #[test]
    fn test_stats_after_eviction() {
        let mut w: RollingWindow<f64> = RollingWindow::new(3);

        w.push(100.0);
        w.push(200.0);
        w.push(300.0);
        w.push(400.0); // evicts 100.0 -> window: [200, 300, 400]

        let s = w.stats().unwrap();
        assert_eq!(s.min, 200.0);
        assert_eq!(s.max, 400.0);
        assert_eq!(s.sum, 900.0);
    }

    #[test]
    fn test_generic_with_i32() {
        // Works with integer types too — T: Copy + Into<f64>
        let mut w: RollingWindow<i32> = RollingWindow::new(5);
        for i in 1..=5 { w.push(i); }

        let s = w.stats().unwrap();
        assert_eq!(s.mean, 3.0);  // (1+2+3+4+5)/5
        assert_eq!(s.min,  1.0);
        assert_eq!(s.max,  5.0);
    }

    #[test]
    fn test_count_above() {
        let mut w: RollingWindow<f64> = RollingWindow::new(5);
        [100.0, 200.0, 500.0, 600.0, 150.0].iter().for_each(|&v| w.push(v));

        // Values above 300ms threshold: [500, 600] → 2
        assert_eq!(w.count_above(300.0), 2);
    }

    #[test]
    fn test_mean_convenience() {
        let mut w: RollingWindow<f64> = RollingWindow::new(4);
        [10.0, 20.0, 30.0, 40.0].iter().for_each(|&v| w.push(v));

        assert_eq!(w.mean(), Some(25.0));
    }
}
