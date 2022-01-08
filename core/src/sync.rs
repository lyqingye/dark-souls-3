use crate::error::ProcessError;
use crate::utf8_str;
use anyhow::{anyhow, Result};
use std::cell::UnsafeCell;
use std::ffi::CString;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use winapi::shared::ntdef::CSTRING;

use windows::Win32::Foundation::{CloseHandle, BOOL, HANDLE};
use windows::Win32::System::Threading::{
    CreateMutexExA, CreateMutexW, ReleaseMutex, WaitForSingleObject, CREATE_MUTEX_INITIAL_OWNER,
    WAIT_ABANDONED, WAIT_OBJECT_0,
};

pub struct Mutex<T: ?Sized> {
    handle: HANDLE,
    data: UnsafeCell<T>,
}

impl<T> Debug for Mutex<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*format!("{}", self.handle.0))
    }
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: &'a Mutex<T>,
}

unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T> Mutex<T> {
    pub fn new(name: &str, guard: T) -> Result<Self> {
        let mut mutex_handle: HANDLE = HANDLE::default();
        while mutex_handle.is_invalid() {
            unsafe {
                mutex_handle = CreateMutexExA(
                    std::ptr::null_mut(),
                    utf8_str!(format!("Global\\{}", name)),
                    0,
                    0x1F0001,
                );
            }
        }
        let mutex = Self {
            handle: mutex_handle,
            data: UnsafeCell::new(guard),
        };
        Ok(mutex)
    }
}

impl<T: ?Sized> Mutex<T> {
    pub fn acquire(&self) -> Result<MutexGuard<'_, T>> {
        self.try_acquire(std::time::Duration::MAX)
    }

    pub fn try_acquire(&self, timeout: std::time::Duration) -> Result<MutexGuard<'_, T>> {
        unsafe {
            match WaitForSingleObject(self.handle, timeout.as_millis() as u32) {
                WAIT_OBJECT_0 => Ok(MutexGuard::new(self)),
                WAIT_ABANDONED => Err(anyhow!(
                    "A thread holding the mutex has left it in a poisened state"
                )),
                _ => Err(anyhow!("Failed to acquire lock")),
            }
        }
    }

    pub fn release(&self) {
        unsafe {
            ReleaseMutex(self.handle);
        }
    }
}

impl<T: ?Sized> Drop for Mutex<T> {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle) };
    }
}

impl<'mutex, T: ?Sized> MutexGuard<'mutex, T> {
    unsafe fn new(lock: &'mutex Mutex<T>) -> MutexGuard<'mutex, T> {
        MutexGuard { lock }
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.lock.release() }
    }
}
