use parking_lot::lock_api::RawMutex as RMutex;

use super::RawSimpleLock;

pub struct RawMutex {
    inner: parking_lot::RawMutex,
}

unsafe impl RawSimpleLock for RawMutex {
    #[inline]
    fn new() -> Self {
        Self {
            inner: RMutex::INIT,
        }
    }

    #[inline]
    fn try_lock(&self) -> bool {
        self.inner.try_lock()
    }

    #[inline]
    fn lock(&self) {
        self.inner.lock();
    }

    #[inline]
    fn unlock(&self) {
        unsafe { self.inner.unlock() };
    }
}
