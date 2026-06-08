// usize = the "indexing integer" typeAlsways the right pointer size
// for the platform (64-bit on modern machines). Use it for lengths,
// indices, and counts - never i32 for these

// The <T> makes the generic - works for String, u32, LogLine, anything.
// Think Python: class CircularBuffer(Generic[T])
pub struct CircularBuffer<T> {
    // Vec<T> owns its data on the heap. the struct owns the Vec.
    // When CircularBuffer is dropped, Vec is dropped, heap is freed
    data: Vec<T>,

    // How many slots exist total - set at construction, never changes.
    capacity: usize,

    // How many slots are currently filled (0..=capacity).
    len: usize,

    // Index of the OLDEST item - where we read from, where we overwrite next
    // This is what makes it a "ring" - it wraps around modulo capacity
    head: usize,
}

impl<T> CircularBuffer<T> {
    // Associated function - no 'self'. Called as CicrularBuffer::new(4).
    // Returns Self (= CircularBuffer<T>). No Result here because a
    // capacity of 0 is a programming error, so we panic early.
    pub fn new(capacity: usize) -> Self {
        // Guard: a buffer with 0 slots makes no sense.
        // assert! panics with a message if the condition is false
        // Bettern than silently creating a broken struct
        assert!(capacity > 0, "CircularBuffer capacity. must by > 0");

        // Vec::with_capacity pre-allocates memory on the heap but
        // sets len to 0 - no items yet, just reserved space.
        // Python analogy: [None] * capacity but without filling it.
        Self {
            data: Vec::with_capacity(capacity),
            capacity,   // shorthand for capacity: capacity (field == var name)
            len: 0,     // starts empty
            head: 0,    // oldest item will be at index 0 initially
        }
    }

    // Simple read-only accessors - &self means "borrow me, don't own me"
    // Python: @property methods

    // How many items are currently stored?
    pub fn len(&self) -> usize {
        self.len
    }

    // Is the buffer at full capacity?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    // Is every slot filled? (next push will overwrite oldest)
    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    // &mut self - we're CHANGING the buffer, so we need mutable borrow.
    // item: T - we take OWNERSHIP of the item. The buffer now owns it.
    pub fn push(&mut self, item: T) {
        // Calculate where to write: treat the array as a ring.
        // (head + len) % capacity always gives the next write slot.
        // Example: head=2, len=4, cap=4 -> (2+4)%4=2
        let write_idx = (self.head + self.len) % self.capacity;

        if self.is_full() {
            // Buffer is at capacity - overwrite the oldest item.
            // In Rust, we must explicitly replace the value because
            // the old item at write_idx still exists and owns its data.
            // data[write_idx] = item would drop the old value cleanly.
            self.data[write_idx] = item;

            // Advance head: oldest is now the NEXT slot.
            // % capacity handles wrapping: if head was 3 and cap is 4,
            // new head is 0 (wraps back to start).
            self.head = (self.head + 1) % self.capacity;
            //len stays the same - still full.
        } else {
            // Buffer has room - just push, don't overwrite.
            // Vec::push appends and increments Vec's internal len.
            self.data.push(item);
            // Track our own len counter too.
            self.len += 1;
        }
    }

    // Read an item by logical index (0 = oldest, len-1 = newest).
    // Returns Option<&T> - might be out of bounds, so we return None
    // instead of panicking. &T = borrow, not own - caller just looks.
    pub fn get(&self, index: usize) -> Option<&T> {
        // Bounds check - return None if index is out of range.
        // This is idiomatic Rust: never panic when None makes sense.
        if index >= self.len {
            return None;
        }

        // Convert logical index (0=oldest) to physical array index.
        // head points to oldest. Offset by index, wrap with modulo.
        // Example: head=2, cap=4, logical index -> physical (2+1)%4=3
        let physical_idx = (self.head + index) % self.capacity;

        // Return a reference to the item. Some(&T) wraps it in Option.
        // Caller uses .unwrap(), .expect(), or pattern matching to get it.
        Some(&self.data[physical_idx])
    }

    // Iterate all items oldest -> newest. Returns a Vec of references.
    // &T borrows each item - caller can read but not mutate or own.
    pub fn iter(&self) -> Vec<&T> {
        // (0..self.len) creates a range - like Python's range(self.len)
        (0..self.len)
            // .map() transforms each index into a reference via get()
            // .unwrap() is safe here because we bounded by self.len above
            .map(|i| self.get(i).unwrap())
            // .collect() gathers the iterator into a Vec
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;   // import everything from the parent module

    #[test]
    fn test_basic_push_and_get() {
        // Build a buffer with capacity 3
        let mut buf: CircularBuffer<i32> = CircularBuffer::new(3);

        buf.push(1);
        buf.push(2);
        buf.push(3);

        // get(0) = oldest item = 1
        assert_eq!(buf.get(0), Some(&1));
        assert_eq!(buf.get(2), Some(&3));
        assert_eq!(buf.len(), 3);
        assert!(buf.is_full());
    }

    #[test]
    fn test_overwrite_when_full() {
        let mut buf: CircularBuffer<i32> = CircularBuffer::new(3);

        buf.push(1);
        buf.push(2);
        buf.push(3);
        buf.push(4);    // overwrites 1

        // Oldest is now 2, newest is 4
        assert_eq!(buf.get(0), Some(&2));
        assert_eq!(buf.get(2), Some(&4));
        assert_eq!(buf.len(), 3);   // still 3, not 4
    }

    #[test]
    fn test_out_of_bounds_returns_none() {
        let mut buf: CircularBuffer<i32> = CircularBuffer::new(3);
        buf.push(10);

        assert_eq!(buf.get(1), None);   // only 1 item, index 1 is out of bounds
        assert_eq!(buf.get(99), None);  // way out of bounds
    }
}
