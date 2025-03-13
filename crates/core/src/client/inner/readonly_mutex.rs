use std::ops::Deref;
use std::sync::{Arc, Mutex, MutexGuard};

// Read-only guard that wraps a MutexGuard
pub struct ReadGuard<'a, T> {
    guard: MutexGuard<'a, T>,
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

/// A read-only wrapper around Arc<Mutex<T>>
pub struct ReadOnlyMutex<T> {
    inner: Arc<Mutex<T>>,
}

impl<T> ReadOnlyMutex<T> {
    pub fn new(mutex: Arc<Mutex<T>>) -> Self {
        Self { inner: mutex }
    }

    pub fn read(&self) -> ReadGuard<'_, T> {
        ReadGuard {
            guard: self.inner.lock().unwrap(),
        }
    }
}

impl<T> Clone for ReadOnlyMutex<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
