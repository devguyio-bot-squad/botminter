//! Work-item lock — ensures exactly one session processes a given work item at a time.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;

use super::types::SessionId;

/// Per-work-item mutex: maps work_item_id → owning session_id.
///
/// `acquire` is non-blocking: it immediately returns `true` if the caller won, `false` if already held.
/// Callers must call `release_all` when their session terminates.
#[derive(Clone)]
pub struct WorkItemLock {
    locks: Arc<Mutex<HashMap<String, SessionId>>>,
}

impl WorkItemLock {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Try to acquire the lock for `work_item_id` on behalf of `session_id`.
    ///
    /// Returns `Ok(true)` if the lock was acquired, `Ok(false)` if already held by another session.
    /// Idempotent: re-acquiring an already-owned lock returns `Ok(true)`.
    pub fn acquire(&self, work_item_id: &str, session_id: &SessionId) -> Result<bool> {
        let mut locks = self.locks.lock().unwrap();
        match locks.get(work_item_id) {
            None => {
                locks.insert(work_item_id.to_string(), session_id.clone());
                Ok(true)
            }
            Some(existing) if existing == session_id => Ok(true),
            Some(_) => Ok(false),
        }
    }

    /// Release the lock for `work_item_id` held by `session_id`.
    ///
    /// Returns an error if the lock is not held by `session_id`.
    pub fn release(&self, work_item_id: &str, session_id: &SessionId) -> Result<()> {
        let mut locks = self.locks.lock().unwrap();
        match locks.get(work_item_id) {
            Some(existing) if existing == session_id => {
                locks.remove(work_item_id);
                Ok(())
            }
            Some(_) => Err(anyhow::anyhow!(
                "lock '{}' is not held by session {}",
                work_item_id,
                session_id
            )),
            None => Err(anyhow::anyhow!("lock '{}' is not held", work_item_id)),
        }
    }

    /// Release all locks held by `session_id`. Called on session termination.
    pub fn release_all(&self, session_id: &SessionId) -> Result<()> {
        let mut locks = self.locks.lock().unwrap();
        locks.retain(|_, v| v != session_id);
        Ok(())
    }

    /// Return the number of currently held locks.
    #[cfg(test)]
    pub(crate) fn held_count(&self) -> usize {
        self.locks.lock().unwrap().len()
    }
}

impl Default for WorkItemLock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    // AC-13: acquire returns true for a new lock
    #[test]
    fn acquire_returns_true_for_new_work_item() {
        let lock = WorkItemLock::new();
        let session = SessionId::new();
        let won = lock
            .acquire("issue-42", &session)
            .expect("acquire must not error on first call");
        assert!(won, "first acquire on a new work item must return true");
    }

    // AC-13: acquire returns false when already held by a different session
    #[test]
    fn acquire_returns_false_when_already_locked_by_other() {
        let lock = WorkItemLock::new();
        let s1 = SessionId::new();
        let s2 = SessionId::new();

        let won_first = lock.acquire("issue-42", &s1).expect("first acquire");
        assert!(won_first, "first acquirer must win");

        let won_second = lock.acquire("issue-42", &s2).expect("second acquire");
        assert!(
            !won_second,
            "second acquirer must lose when lock is already held"
        );
    }

    // AC-13: acquire is idempotent for the same session
    #[test]
    fn acquire_same_item_same_session_is_idempotent() {
        let lock = WorkItemLock::new();
        let session = SessionId::new();

        let first = lock.acquire("issue-99", &session).expect("first acquire");
        assert!(first, "first call must succeed");

        let second = lock
            .acquire("issue-99", &session)
            .expect("idempotent acquire");
        assert!(
            second,
            "same session re-acquiring its own lock must return true"
        );
    }

    // AC-14a: release allows the lock to be re-acquired
    #[test]
    fn release_allows_reacquire_by_other_session() {
        let lock = WorkItemLock::new();
        let s1 = SessionId::new();
        let s2 = SessionId::new();

        lock.acquire("issue-7", &s1).expect("acquire by s1");
        lock.release("issue-7", &s1).expect("release by s1");

        let won = lock
            .acquire("issue-7", &s2)
            .expect("acquire by s2 after release");
        assert!(
            won,
            "after release, another session must be able to acquire"
        );
    }

    // AC-14a: release by non-owner returns an error
    #[test]
    fn release_by_non_owner_returns_error() {
        let lock = WorkItemLock::new();
        let owner = SessionId::new();
        let interloper = SessionId::new();

        lock.acquire("issue-55", &owner).expect("acquire");
        let result = lock.release("issue-55", &interloper);
        assert!(
            result.is_err(),
            "release by a non-owning session must return an error"
        );
    }

    // AC-14b: release_all releases all locks held by a session
    #[test]
    fn release_all_clears_all_locks_for_session() {
        let lock = WorkItemLock::new();
        let session = SessionId::new();

        lock.acquire("item-1", &session).expect("acquire item-1");
        lock.acquire("item-2", &session).expect("acquire item-2");
        lock.acquire("item-3", &session).expect("acquire item-3");
        assert_eq!(lock.held_count(), 3, "three locks must be held");

        lock.release_all(&session).expect("release_all");
        assert_eq!(
            lock.held_count(),
            0,
            "all locks must be released after release_all"
        );
    }

    // AC-14b: release_all is a no-op for a session holding no locks
    #[test]
    fn release_all_is_noop_for_session_with_no_locks() {
        let lock = WorkItemLock::new();
        let session = SessionId::new();

        lock.release_all(&session)
            .expect("release_all with no locks must not error");
        assert_eq!(lock.held_count(), 0);
    }

    // AC-13 (concurrency): Two sessions racing to acquire the same item — exactly one wins
    #[test]
    fn concurrent_acquire_exactly_one_winner() {
        let lock = Arc::new(WorkItemLock::new());
        let wins = Arc::new(AtomicUsize::new(0));
        let losses = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..10 {
            let lock_clone = Arc::clone(&lock);
            let wins_clone = Arc::clone(&wins);
            let losses_clone = Arc::clone(&losses);
            let handle = thread::spawn(move || {
                let session = SessionId::new();
                match lock_clone.acquire("contested-item", &session) {
                    Ok(true) => {
                        wins_clone.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(false) => {
                        losses_clone.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(e) => panic!("acquire must not error: {e}"),
                }
            });
            handles.push(handle);
        }

        for h in handles {
            h.join().expect("thread must not panic");
        }

        assert_eq!(
            wins.load(Ordering::SeqCst),
            1,
            "exactly one session must win the lock"
        );
        assert_eq!(
            losses.load(Ordering::SeqCst),
            9,
            "all other sessions must lose"
        );
    }

    // AC-14a: Two sessions for the same member scanning the same board — exactly one holds the lock
    #[test]
    fn two_sessions_same_work_item_only_one_proceeds() {
        let lock = WorkItemLock::new();
        let s1 = SessionId::new();
        let s2 = SessionId::new();

        let s1_won = lock.acquire("board-scan:item-99", &s1).expect("s1 acquire");
        let s2_won = lock.acquire("board-scan:item-99", &s2).expect("s2 acquire");

        // Exactly one must hold the lock
        assert!(
            s1_won ^ s2_won,
            "exactly one of s1 or s2 must hold the lock, got s1={s1_won} s2={s2_won}"
        );
    }
}
