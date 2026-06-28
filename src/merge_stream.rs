use std::collections::BinaryHeap;

// LogRecord: one entry from a log stream
// Field order matters for #[derive(Ord)] - Rust compares top to bottom
// timestamp FIRST = records sorted primarily by time
// stream_id SECOND = stable tiebreaker (deterministic output)
// message LAST = content, not part of ordering
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct LogRecord {
    // Primary sort key: Unix timestamp in milliseconds
    // Must be FIRST field for derive(Ord) to sort by it first
    pub timestamp: u64,

    // Tiebreaker: which stream this record came from
    // Makes sort stable when two records share a timestamp
    pub stream_id: usize,

    // The actual log message. Compared last - rarely affects order
    pub message: String,
}

impl LogRecord {
    pub fn new(timestamp: u64, stream_id: usize, message: &str) -> Self {
        Self {
            timestamp,
            stream_id,
            message: message.to_string(),
        }
    }
}

// HeapItem: what we store in the BinaryHeap
// A manual Ord impl (below) reverses the comparison for min-heap behavior
// so the SMALLEST timestamp sorts as "largest" in the heap and pops first
// We need stream_idx to know which stream to pull the next record from
// after popping this one from the heap
#[derive(Debug, Eq, PartialEq)]
struct HeapItem {
    record: LogRecord,
    stream_idx: usize, // which input stream this came from
}

// Implement Ord on HeapItem - delegates to the record's Ord
// We want MIN-heap (smallest timestamp first), so we REVERSE the comparison
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // other.record.cmp(self.record) - reversed!
        // Largest timestamp -> "smallest" in the heap -> popped last
        // Smallest timestamp -> "largest" in the heap -> popped first
        other.record.cmp(&self.record)
    }
}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// merge_k_streams: takes K sorted iterators, returns one sorted iterator
//
// Input: Vec of Vec<LogRecord> - K streams, each pre-sorted by timestamp
// Output: Vec<LogRecord> - all records globally sorted by timestamp
//
// For a production streaming version, we'd return impl Iterator
// For the interview, returning Vec is simpler and equally correct
//
// Time: O(N log K) - N total records, K streams, log K per heap op
// Space: O(K) - heap holds at most one record per stream
pub fn merge_k_streams(streams: Vec<Vec<LogRecord>>) -> Vec<LogRecord> {
    // Min-heap: holds one candidate record per stream
    // BinaryHeap is a max-heap - HeapItem's Ord is reversed for min behavior
    let mut heap: BinaryHeap<HeapItem> = BinaryHeap::new();

    // Convert each stream into a Peekable iterator
    // Peekable lets us look at the next record without consuming it
    // Vec::into_iter() moves the Vec, giving us ownership of each record
    let mut iters: Vec<_> = streams
        .into_iter()
        .map(|s| s.into_iter().peekable())
        .collect();

    // Seed the heap: push the FIRST record from each stream
    // This gives every stream its initial "candidate" in the heap
    for (stream_idx, iter) in iters.iter_mut().enumerate() {
        // .next() consumes and returns the first record (Option<LogRecord>)
        if let Some(record) = iter.next() {
            heap.push(HeapItem { record, stream_idx });
        }
        // Streams that start empty are simply skipped - no candidate pushed
    }

    // Output buffer: collect merged records here
    let mut output = Vec::new();

    // Main merge loop: runs until the heap is empty (all streams exhausted)
    while let Some(HeapItem { record, stream_idx }) = heap.pop() {
        // Pop the globally minimum record (smallest timestamp)
        // Destructure HeapItem to get the record and which stream it came from
        output.push(record);

        // Refill: push the NEXT record from the same stream that just contributed
        // This maintains the invariant: heap has one candidate per active stream
        if let Some(next_record) = iters[stream_idx].next() {
            heap.push(HeapItem {
                record: next_record,
                stream_idx,
            });
        }
        // If that stream is exhausted, we just don't push - heap shrinks by one
        // When all streams are exhausted, heap empties and the loop ends
    }

    output
}

// MergeIter: a struct that implements Iterator<Item = LogRecord>
// Owns the heap and the per-stream iterators
// Each call to .next() pops one record from the heap and refills
pub struct MergeIter {
    heap: BinaryHeap<HeapItem>,
    // Box<dyn Iterator> because each stream's concrete type is different
    // we can't spell out the type, so we erase it behind a trait object
    iters: Vec<Box<dyn Iterator<Item = LogRecord>>>,
}

impl MergeIter {
    // Constructor: seeds the heap with the first record from each stream
    pub fn new(streams: Vec<Vec<LogRecord>>) -> Self {
        let mut heap = BinaryHeap::new();
        // Convert each Vec into a boxed trait object
        // Box<dyn Iterator> erases the concrete Vec::IntoIter type
        let mut iters: Vec<Box<dyn Iterator<Item = LogRecord>>> = streams
            .into_iter()
            .map(|s| Box::new(s.into_iter()) as Box<dyn Iterator<Item = LogRecord>>)
            .collect();

        // Seed: one candidate per stream into the heap
        for (stream_idx, iter) in iters.iter_mut().enumerate() {
            if let Some(record) = iter.next() {
                heap.push(HeapItem { record, stream_idx });
            }
        }

        Self { heap, iters }
    }
}

// Implement Iterator for MergeIter
// Each .next() call pops one record and refills from the same stream
impl Iterator for MergeIter {
    type Item = LogRecord;

    fn next(&mut self) -> Option<LogRecord> {
        // Pop the minimum record from the heap
        // Returns None when heap is empty -> iterator exhausted
        let HeapItem { record, stream_idx } = self.heap.pop()?;
        // \? propagates None - clean early return when empty

        // Refill the heap from the stream that just contributed
        if let Some(next) = self.iters[stream_idx].next() {
            self.heap.push(HeapItem { record: next, stream_idx });
        }

        Some(record)
    }
}

// Convenience constructor - returns impl Iterator (hides MergeIter type)
// Callers just use it as an iterator - no need to know it's a MergeIter
pub fn merge_streaming(streams: Vec<Vec<LogRecord>>) -> impl Iterator<Item = LogRecord> {
    MergeIter::new(streams)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn make_stream(stream_id: usize, timestamps: &[u64]) -> Vec<LogRecord> {
        timestamps
            .iter()
            .map(|&ts| LogRecord::new(ts, stream_id, &format!("msg-{}", ts)))
            .collect()
    }

    #[test]
    fn test_merge_two_streams() {
        let streams = vec![
            make_stream(0, &[1, 3, 5, 7]),
            make_stream(1, &[2, 4, 6, 8]),
        ];

        let result = merge_k_streams(streams);
        let timestamps: Vec<u64> = result
            .iter()
            .map(|r| r.timestamp)
            .collect();
        assert_eq!(timestamps, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_merge_three_streams() {
        let streams = vec![
            make_stream(0, &[5, 15, 25]),
            make_stream(1, &[8, 12]),
            make_stream(2, &[3, 20]),
        ];

        let result = merge_k_streams(streams);
        let timestamps: Vec<u64> = result.iter().map(|r| r.timestamp).collect();
        assert_eq!(timestamps, vec![3, 5, 8, 12, 15, 20, 25]);
    }

    #[test]
    fn test_merge_with_empty_stream() {
        let streams = vec![
            make_stream(0, &[1, 2, 3]),
            make_stream(1, &[]),          // empty stream — skipped cleanly
            make_stream(2, &[4, 5, 6]),
        ];

        let result = merge_k_streams(streams);
        let timestamps: Vec<u64> = result.iter().map(|r| r.timestamp).collect();
        assert_eq!(timestamps, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_merge_duplicate_timestamps() {
        // Same timestamp from different streams — tiebreak by stream_id
        let streams = vec![
            make_stream(0, &[10, 20]),
            make_stream(1, &[10, 30]),
        ];

        let result = merge_k_streams(streams);
        // Both ts=10s present; stream_id=0 sorts before stream_id=1
        assert_eq!(result[0].stream_id, 0);
        assert_eq!(result[1].stream_id, 1);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_streaming_iterator() {
        let streams = vec![
            make_stream(0, &[1, 3]),
            make_stream(1, &[2, 4]),
        ];

        // Use the streaming version - lazy, no full buffer
        let result: Vec<LogRecord> = merge_streaming(streams).collect();
        let timestamps: Vec<u64> = result
            .iter()
            .map(|r| r.timestamp)
            .collect();
        assert_eq!(timestamps, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_single_stream_passthrough() {
        let streams = vec![make_stream(0, &[1, 2, 3, 4, 5])];
        let result = merge_k_streams(streams);
        let timestamps: Vec<u64> = result
            .iter()
            .map(|r| r.timestamp)
            .collect();
        assert_eq!(timestamps, vec![1, 2, 3, 4, 5]);
    }
}