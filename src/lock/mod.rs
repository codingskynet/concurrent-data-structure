pub mod fclock;
pub mod mutex;
pub mod seqlock;
pub mod spinlock;

pub use mutex::RawMutex;
pub use seqlock::SeqLock;
pub use spinlock::RawSpinLock;
pub use spinlock::SpinLock;

pub unsafe trait RawSimpleLock {
    fn new() -> Self;

    /// Non-blocking: Try locking. If succeeding, return true, or false.
    fn try_lock(&self) -> bool;

    /// Blocking: Get locking or wait until getting locking
    fn lock(&self);

    /// Release lock
    fn unlock(&self);
}
