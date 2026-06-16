// HashMap: key -> value store. O(1) average lookup/insert.
use std::collections::HashMap;

// fmt is the module for formatting - Display and Debug traits live here
use std::fmt;

// #[derive] asks the compiler to auto-implement these traits
// Debug -> enables {:?} printing (for tests and dbg!())
// Clone -> enables .clone() (explicit deep copy)
// PartialEq -> enables == comparisons for tests
#[derive(Debug, Clone, PartialEq)]
pub struct LogEntry {
    // &'static str would be a string literal - but we own these strings
    // so we use String (heap-allocated, owned).
    pub level: String,          // "ERROR", "WARN", "INFO"
    pub timestamp: String,      // "2024-01-15"
    pub source: String,         // "api_server"
    pub message: String,        // "connection timeout"  
}

// The parser itself - owns a Vec of parsed entries
pub struct LogParser {
    // Vec<LogEntry>: a growable list on the heap.
    // LogParser OWNS every LogEntry inside it.
    entries: Vec<LogEntry>,
}

impl LogParser {
    // Associated function - no self. Returns a fresh, empty parser
    pub fn new() -> Self {
        Self {
            // Vec::new() creates an empty vector - no heap allocation yet
            entries: Vec::new(),
        }
    }

    // parse_line takes a &str - a BORROWED string slice, not an owned String
    // Why &str not String? Because:
    //  1. We don't need to own the input - we just read it
    //  2. &str works on both String and string literals - more flexible
    //  3. Avoids a copy - caller keeps their String, we just borrow it
    //  Python analogy: like accepting str vs bytes - prefer the lighter type
    pub fn parse_line(&mut self, line: &str) -> Option<LogEntry> {
        // .trim() removes leading/trailing whitespace - returns &str (still borrowed)
        let line = line.trim();

        // Skip empty lines - return None (no entry to parse)
        if line.is_empty() {
            return None;
        }

        // Parse format: "[LEVEL] timestamp source: message"
        // .strip_prefix("[") returns Option<&str> - None if line doesn't start with "["
        // \? operator: if None, return None from this function immediately
        // Python analogy: like line.removeprefix("[") but None-safe
        let rest = line.strip_prefix('[')?;

        // Find the closing bracket to extract the level
        // .find() returns Option<usize> - the byte index of ']', or None
        let bracket_end = rest.find(']')?;

        // Slice the level out: &rest[..bracket_end] =  borrowed slice of rest
        // .to_string() converts &str -> String (allocates on the heap, we own it)
        let level = rest[..bracket_end].trim().to_string();

        // Everything after "]" is "timestamp source: message"
        // &rest[bracket_end+2..] skips "] " (2 chars)
        let after_bracket = rest[bracket_end + 2..].trim();

        // Split on whitespace to get [timestamp, "source:", rest...]
        // .splitn(3, ' ') splits into at most 3 parts - like Python's .split(' ', 2)
        let mut parts = after_bracket.splitn(3, ' ');
        // .next() on an iterator returns Option<&str> - the next item or None
        let timestamp = parts.next()?.trim().to_string();

        // source ends in ':' - strip it
        let source_raw = parts.next()?.trim();
        // .trim_end_matches(':') removes trailing colon(s) - returns &str
        let source = source_raw.trim_end_matches(':').to_string();

        // Whatever remains is the message
        let message = parts.next().unwrap_or("").trim().to_string();

        // Build the entry - we OWN all four Strings now
        let entry = LogEntry {
            level,
            timestamp,
            source,
            message,
        };

        // Store a clone in our Vec so the caller can also use the entry
        self.entries.push(entry.clone());

        // Return Some(entry) - successfully parsed
        Some(entry)
    }
}

impl LogParser {
    // &self - read-only borrow. We're just counting, not changing entries.
    // Returns HashMap<String, usize>: level name -> count.
    pub fn count_by_level(&self) -> HashMap<String, usize> {
        // Start with an empty map - HashMap::new() with no pre-allocation
        let mut counts: HashMap<String, usize> = HashMap::new();

        // Iterate over entries - &self.entries gives a borrowed slice
        // for entry in &self.entries -> entry is &LogEntry (borrowed, not moved)
        for entry in &self.entries {
            // The Entry API - the idiomatic way to count in Rust:
            //
            //  counts.entry(key)
            //      .or_insert(0)   <- if key missing, insert 0
            //      += 1            <- then increment whatever's there
            //
            // Python equivalent: counts[level] = counts.get(level, 0) + 1
            // But Entry does it in ONE lookup - no double-hashing.
            //
            // entry.level.clone() - we need an owned String as the key.
            // HashMap keys must be owned. The entry lives in self.entries
            // which is borrowed, so we can't move level out - we clone it.
            *counts.entry(entry.level.clone()).or_insert(0) += 1;
        }
        counts
    }

    // Return the top N sources by error frequency.
    // &self - read-only, we're just reading and sorting.
    pub fn top_n_sources(&self, n: usize, level: &str) -> Vec<(String, usize)> {
        let mut source_counts: HashMap<String, usize> = HashMap::new();

        // Filter to only entries matching the requested level
        for entry in &self.entries {
            // entry.level is a String, level is a &str
            // == works between String and &str - Rust coerces automatically
            if entry.level == level {
                *source_counts.entry(entry.source.clone()).or_insert(0) += 1;
            }
        }

        // Convert HashMap into a Vec of (key, value) tuples so we can sort
        // .into_iter() - MOVES the HashMap, consuming it (we don't need it after)
        let mut pairs: Vec<(String, usize)> = source_counts.into_iter().collect();

        // Sort by count descending - sort_by takes a closure (anonymous fn)
        // a.1 = the usize (count). b.cmp(&a) reverses the natural order (descending)
        // Python equivalent: sorted(pairs, key=lambda x: x[1], reverse=True)
        pairs.sort_by(|a, b| b.1.cmp(&a.1));

        // Take at most n items - .truncate() removes elements past index n
        pairs.truncate(n);

        pairs
    }
}

// fmt::Display is the trait for human-readable output.
// Implementing it means println!("{}", parser) works.
// Python analogy: def __str__(self) on a class
impl fmt::Display for LogParser {
    // fmt is the formatter - write to it with write!() macro
    // Returns fmt::Result - Ok(()) on success, Err on failure
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //write!() to f is like print but into the formatter buffer
        write!(f, "=== Log Summary ({} entries) ===\n", self.entries.len())?;

        // Get counts and display each level
        let counts = self.count_by_level();

        // Sort levels alphabetically for consistent output
        let mut levels: Vec<&String> = counts.keys().collect();
        levels.sort();

        for level in levels {
            // count[level] - index operator on HashMap, panics if missing
            // Using get() would return Option but we know it exists
            write!(f, " {:<6} : {} entries\n", level, counts[level])?;
        }

        // Show top 3 error sources
        let top = self.top_n_sources(3, "ERROR");
        if !top.is_empty() {
            write!(f, "--- Top ERROR sources ---\n")?;
            for (i, (source, count)) in top.iter().enumerate() {
                // enumerate() gives (index, value) - like Python's enumerate()
                write!(f, " {}. {} ({} errors)\n", i + 1, source, count)?;
            }
        }

        Ok(())  // Signal success - no error
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_line() {
        let mut parser = LogParser::new();
        let entry = parser.parse_line("[ERROR] 2024-01-15 api_server: connection timeout");

        assert!(entry.is_some());
        let e = entry.unwrap();
        assert_eq!(e.level, "ERROR");
        assert_eq!(e.source, "api_server");
        assert_eq!(e.message, "connection timeout");
    }

    #[test]
    fn test_count_by_level() {
        let mut parser = LogParser::new();
        parser.parse_line("[ERROR] 2024-01-15 api_server: timeout");
        parser.parse_line("[ERROR] 2024-01-15 db_server: timeout");
        parser.parse_line("[WARN]  2024-01-15 api_server: slow response");
        parser.parse_line("[INFO]  2024-01-15 api_server: started");

        let counts = parser.count_by_level();
        assert_eq!(counts["ERROR"], 2);
        assert_eq!(counts["WARN"], 1);
        assert_eq!(counts["INFO"], 1);
    }

    #[test]
    fn test_top_n_sources() {
        let mut parser = LogParser::new();
        parser.parse_line("[ERROR] 2024-01-15 api_server: err1");
        parser.parse_line("[ERROR] 2024-01-15 api_server: err2");
        parser.parse_line("[ERROR] 2024-01-15 db_server: err3");

        let top = parser.top_n_sources(1, "ERROR");
        assert_eq!(top[0].0, "api_server");
        assert_eq!(top[0].1, 2);
    }

    #[test]
    fn test_display_trait() {
        let mut parser = LogParser::new();
        parser.parse_line("[ERROR] 2024-01-15 api_server: timeout");

        // If Display is implemented, this compiles and runs
        let output = format!("{}", parser);
        assert!(output.contains("Log Summary"));
        assert!(output.contains("ERROR"));
    }    
}