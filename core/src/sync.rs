use crate::error::ProcessError;
use crate::utf16_str;
use anyhow::Result;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winnt::SYNCHRONIZE;
use windows::Win32::Foundation::{BOOL, HANDLE};
use windows::Win32::System::Threading::{
    CreateMutexW, OpenMutexW, ReleaseMutex, WaitForSingleObject,
};

pub struct Mutex {
    handle: HANDLE,
    unlocked: bool,
}

impl Mutex {
    pub fn acquire(name: &str) -> Result<Self> {
        unsafe {
            let full_name = format!("{}", name);
            let handle = CreateMutexW(std::ptr::null(), BOOL::from(false), utf16_str!(full_name));
            if handle.is_invalid() {
                return Err(ProcessError::CreateMutex {
                    source: std::io::Error::last_os_error(),
                }
                .into());
            }
            WaitForSingleObject(handle, u32::MAX);
            Ok(Self {
                handle,
                unlocked: false,
            })
        }
    }

    pub fn release(&mut self) {
        if !self.unlocked {
            unsafe {
                ReleaseMutex(self.handle);
            }
            self.unlocked = true;
        }
    }
}

impl Drop for Mutex {
    fn drop(&mut self) {
        self.release()
    }
}
