/* This code from https://github.com/kaist-cp/cs431/blob/main/lock/src/seqlock.rs
 */

use core::mem;
use core::ops::Deref;
use core::sync::atomic::{fence, AtomicUsize, Ordering};

use crossbeam_utils::Backoff;

#[derive(Debug)]
struct RawSeqLock {
    seq: AtomicUsize,
}

impl RawSeqLock {
    const fn new() -> Self {
        Self {
            seq: AtomicUsize::new(0),
        }
    }

    fn write_lock(&self) -> usize {
        let backoff = Backoff::new();

        loop {
            let seq = self.seq.load(Ordering::Relaxed);
            if seq & 1 == 0
                && self
                    .seq
                    .compare_exchange(
                        seq,
                        seq.wrapping_add(1),
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    )
                    .is_ok()
            {
                fence(Ordering::Release);
                return seq;
            }

            backoff.snooze();
        }
    }

    fn write_unlock(&self, seq: usize) {
        self.seq.store(seq.wrapping_add(2), Ordering::Release);
    }

    fn read_begin(&self) -> usize {
        let backoff = Backoff::new();

        loop {
            let seq = self.seq.load(Ordering::Acquire);
            if seq & 1 == 0 {
                return seq;
            }

            backoff.snooze();
        }
    }

    fn read_validate(&self, seq: usize) -> bool {
        fence(Ordering::Acquire);

        seq == self.seq.load(Ordering::Relaxed)
    }

    unsafe fn upgrade(&self, seq: usize) -> Result<(), ()> {
        if self
            .seq
            .compare_exchange(
                seq,
                seq.wrapping_add(1),
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_err()
        {
            return Err(());
        }

        fence(Ordering::Release);
        Ok(())
    }
}

#[derive(Debug)]
pub struct SeqLock<T> {
    lock: RawSeqLock,
    data: T,
}

#[derive(Debug)]
pub struct WriteGuard<'s, T> {
    lock: &'s SeqLock<T>,
    seq: usize,
}

#[derive(Debug)]
pub struct ReadGuard<'s, T> {
    lock: &'s SeqLock<T>,
    seq: usize,
}

unsafe impl<T: Send> Send for SeqLock<T> {}
unsafe impl<T: Send> Sync for SeqLock<T> {}

unsafe impl<'s, T> Send for WriteGuard<'s, T> {}
unsafe impl<'s, T: Send + Sync> Sync for WriteGuard<'s, T> {}

unsafe impl<'s, T> Send for ReadGuard<'s, T> {}
unsafe impl<'s, T: Send + Sync> Sync for ReadGuard<'s, T> {}

impl<T> SeqLock<T> {
    pub fn new(data: T) -> Self {
        SeqLock {
            lock: RawSeqLock::new(),
            data: data.into(),
        }
    }

    pub fn write_lock(&self) -> WriteGuard<T> {
        let seq = self.lock.write_lock();
        WriteGuard { lock: self, seq }
    }

    pub unsafe fn read_lock(&self) -> ReadGuard<T> {
        let seq = self.lock.read_begin();
        ReadGuard { lock: self, seq }
    }

    pub unsafe fn read<F, R>(&self, f: F) -> Result<R, ()>
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.read_lock();
        let result = f(&guard);

        if guard.finish() {
            Ok(result)
        } else {
            Err(())
        }
    }
}

impl<'s, T> Deref for WriteGuard<'s, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.lock.data
    }
}

impl<'s, T> Drop for WriteGuard<'s, T> {
    fn drop(&mut self) {
        self.lock.lock.write_unlock(self.seq);
    }
}

impl<'s, T> Deref for ReadGuard<'s, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.lock.data
    }
}

impl<'s, T> Clone for ReadGuard<'s, T> {
    fn clone(&self) -> Self {
        Self {
            lock: self.lock,
            seq: self.seq,
        }
    }
}

impl<'s, T> ReadGuard<'s, T> {
    pub fn validate(&self) -> bool {
        self.lock.lock.read_validate(self.seq)
    }

    pub fn restart(&mut self) {
        self.seq = self.lock.lock.read_begin();
    }

    pub fn finish(self) -> bool {
        let result = self.lock.lock.read_validate(self.seq);
        mem::forget(self);
        result
    }

    pub fn upgrade(self) -> Result<WriteGuard<'s, T>, ()> {
        let result = if unsafe { self.lock.lock.upgrade(self.seq).is_ok() } {
            Ok(WriteGuard {
                lock: self.lock,
                seq: self.seq,
            })
        } else {
            Err(())
        };
        mem::forget(self);
        result
    }
}
