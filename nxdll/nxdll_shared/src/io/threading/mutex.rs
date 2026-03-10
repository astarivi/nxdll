use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::mem;
use nxdk_rs::sys::winapi::*;

pub struct Mutex<T> {
    cs: UnsafeCell<CRITICAL_SECTION>,
    data: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> {}
unsafe impl<T> Send for Mutex<T> {}

impl<T> Mutex<T> {
    /// Create a new Mutex. CRITICAL_SECTION will be initialized.
    pub fn new(data: T) -> Self {
        let cs = UnsafeCell::new(unsafe { mem::zeroed() });
        let mutex = Mutex {
            cs,
            data: UnsafeCell::new(data),
        };
        unsafe { InitializeCriticalSection(mutex.cs.get()) };
        mutex
    }

    pub fn lock(&self) -> MutexGuard<T> {
        unsafe { EnterCriticalSection(self.cs.get()) };
        MutexGuard { mutex: self }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        let result = unsafe { TryEnterCriticalSection(self.cs.get()) };
        if result != 0 {
            Some(MutexGuard { mutex: self })
        } else {
            None
        }
    }
}

impl<T> Drop for Mutex<T> {
    fn drop(&mut self) {
        unsafe { DeleteCriticalSection(self.cs.get()) };
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe { LeaveCriticalSection(self.mutex.cs.get()) };
    }
}