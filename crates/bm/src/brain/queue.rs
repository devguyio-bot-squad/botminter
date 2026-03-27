use std::collections::BinaryHeap;
use std::cmp::Ordering;

use super::types::BrainMessage;

/// Wrapper for BinaryHeap ordering: highest priority (lowest enum value) first.
/// Ties are broken by insertion order (FIFO within same priority).
#[derive(Debug)]
struct QueueEntry {
    message: BrainMessage,
    /// Monotonically increasing sequence number for FIFO within same priority.
    seq: u64,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.message.priority == other.message.priority && self.seq == other.seq
    }
}

impl Eq for QueueEntry {}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap, so we reverse: lower priority value = higher priority.
        // For same priority, lower seq = earlier insertion = should come first (reverse seq).
        other
            .message
            .priority
            .cmp(&self.message.priority)
            .then(other.seq.cmp(&self.seq))
    }
}

/// A priority queue for brain messages.
///
/// Messages are ordered by priority (Human > LoopEvent > Heartbeat),
/// with FIFO ordering within the same priority level.
///
/// # Design
///
/// Uses a `BinaryHeap` with reversed ordering so that `pop()` returns
/// the highest-priority (lowest enum value) message. A monotonic sequence
/// counter ensures FIFO within the same priority.
pub struct PromptQueue {
    heap: BinaryHeap<QueueEntry>,
    next_seq: u64,
}

impl PromptQueue {
    /// Create a new empty prompt queue.
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            next_seq: 0,
        }
    }

    /// Push a message into the queue.
    pub fn push(&mut self, message: BrainMessage) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.heap.push(QueueEntry { message, seq });
    }

    /// Pop the highest-priority message from the queue.
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&mut self) -> Option<BrainMessage> {
        self.heap.pop().map(|entry| entry.message)
    }

    /// Returns `true` if the queue has no messages.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Returns the number of messages in the queue.
    pub fn len(&self) -> usize {
        self.heap.len()
    }
}

impl Default for PromptQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::types::{BrainMessage, Priority};

    #[test]
    fn empty_queue() {
        let mut q = PromptQueue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        assert!(q.pop().is_none());
    }

    #[test]
    fn single_message() {
        let mut q = PromptQueue::new();
        q.push(BrainMessage::human("hi"));
        assert_eq!(q.len(), 1);
        assert!(!q.is_empty());

        let msg = q.pop().unwrap();
        assert_eq!(msg.priority, Priority::Human);
        assert_eq!(msg.content, "hi");
        assert!(q.is_empty());
    }

    #[test]
    fn priority_ordering_human_before_heartbeat() {
        let mut q = PromptQueue::new();
        // Insert heartbeat first, then human
        q.push(BrainMessage::heartbeat());
        q.push(BrainMessage::human("urgent"));

        // Human should come out first despite being inserted second
        let first = q.pop().unwrap();
        assert_eq!(first.priority, Priority::Human);

        let second = q.pop().unwrap();
        assert_eq!(second.priority, Priority::Heartbeat);
    }

    #[test]
    fn priority_ordering_all_three() {
        let mut q = PromptQueue::new();
        // Insert in reverse priority order
        q.push(BrainMessage::heartbeat());
        q.push(BrainMessage::loop_event("loop-1", "build.done", "ok"));
        q.push(BrainMessage::human("help"));

        let first = q.pop().unwrap();
        assert_eq!(first.priority, Priority::Human);

        let second = q.pop().unwrap();
        assert_eq!(second.priority, Priority::LoopEvent);

        let third = q.pop().unwrap();
        assert_eq!(third.priority, Priority::Heartbeat);
    }

    #[test]
    fn fifo_within_same_priority() {
        let mut q = PromptQueue::new();
        q.push(BrainMessage::human("first"));
        q.push(BrainMessage::human("second"));
        q.push(BrainMessage::human("third"));

        assert_eq!(q.pop().unwrap().content, "first");
        assert_eq!(q.pop().unwrap().content, "second");
        assert_eq!(q.pop().unwrap().content, "third");
    }

    #[test]
    fn mixed_priorities_with_fifo() {
        let mut q = PromptQueue::new();
        q.push(BrainMessage::loop_event("l1", "ev1", "s1"));
        q.push(BrainMessage::heartbeat());
        q.push(BrainMessage::human("h1"));
        q.push(BrainMessage::loop_event("l2", "ev2", "s2"));
        q.push(BrainMessage::human("h2"));

        // All humans first (FIFO), then loop events (FIFO), then heartbeat
        let m1 = q.pop().unwrap();
        assert_eq!(m1.priority, Priority::Human);
        assert_eq!(m1.content, "h1");

        let m2 = q.pop().unwrap();
        assert_eq!(m2.priority, Priority::Human);
        assert_eq!(m2.content, "h2");

        let m3 = q.pop().unwrap();
        assert_eq!(m3.priority, Priority::LoopEvent);
        assert!(m3.content.contains("ev1"));

        let m4 = q.pop().unwrap();
        assert_eq!(m4.priority, Priority::LoopEvent);
        assert!(m4.content.contains("ev2"));

        let m5 = q.pop().unwrap();
        assert_eq!(m5.priority, Priority::Heartbeat);

        assert!(q.is_empty());
    }

    #[test]
    fn default_creates_empty_queue() {
        let q = PromptQueue::default();
        assert!(q.is_empty());
    }

    #[test]
    fn interleaved_push_pop() {
        let mut q = PromptQueue::new();
        q.push(BrainMessage::heartbeat());

        let msg = q.pop().unwrap();
        assert_eq!(msg.priority, Priority::Heartbeat);

        q.push(BrainMessage::human("later"));
        let msg = q.pop().unwrap();
        assert_eq!(msg.priority, Priority::Human);
        assert_eq!(msg.content, "later");

        assert!(q.is_empty());
    }
}
