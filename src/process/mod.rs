use anyhow::Result;
use std::{mem, ptr};
use winapi::shared::ntdef::HANDLE;
use winapi::um::memoryapi::{ReadProcessMemory, WriteProcessMemory};
use winapi::shared::minwindef::{FALSE, LPCVOID, LPVOID, BOOL, PBOOL};
use winapi::shared::basetsd::SIZE_T;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::winnt::PROCESS_ALL_ACCESS;
use winapi::um::wow64apiset::IsWow64Process;
use winapi::um::tlhelp32::{Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS, CreateToolhelp32Snapshot};

#[derive(Debug)]
pub struct Process {
    pub id: u32,
    pub is_wow64: bool,
    pub handle: HANDLE,
}

impl Process {
    pub fn read<T: Copy>(&self, address: usize) -> Option<T> {
        let mut buffer = unsafe { mem::zeroed::<T>() };
        match unsafe {
            ReadProcessMemory(
                self.handle,
                address as LPCVOID,
                &mut buffer as *mut T as LPVOID,
                mem::size_of::<T>() as SIZE_T,
                ptr::null_mut::<SIZE_T>(),
            )
        } {
            FALSE => None,
            _ => Some(buffer),
        }
    }

    pub fn read_ptr<T: Copy>(&self, buf: *mut T, address: usize, count: usize) -> bool {
        unsafe {
            ReadProcessMemory(
                self.handle,
                address as LPCVOID,
                buf as *mut T as LPVOID,
                mem::size_of::<T>() as SIZE_T * count,
                ptr::null_mut::<SIZE_T>(),
            ) != FALSE
        }
    }

    pub fn write<T: Copy>(&self, address: usize,buf: &T) -> bool {
        unsafe {
            WriteProcessMemory(
                self.handle,
                address as LPVOID,
                buf as *const T as LPCVOID,
                mem::size_of::<T>() as SIZE_T,
                ptr::null_mut::<SIZE_T>(),
            ) != FALSE
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { CloseHandle(self.handle) };
        }
    }
}

pub fn from_pid(pid: u32) -> Option<Process> {
    let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid)};
    if handle.is_null() {
        return None;
    }

    let mut tmp: BOOL = 0;
    if unsafe { IsWow64Process(handle,&mut tmp as PBOOL) } == FALSE {
        return None;
    }

    let is_wow64 = match tmp {
        FALSE => false,
        _ => true,
    };

    Some(Process {
        id: pid,
        is_wow64,
        handle,
    })
}

pub fn from_name(name: &str) -> Option<Process> {
    let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

    if handle.is_null() {
        return None;
    }

    let mut pe: PROCESSENTRY32W = unsafe { mem::zeroed() };
    pe.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;
    if unsafe { Process32FirstW(handle,&mut pe)} == FALSE {
        return None;
    }

    loop {
        let process_name = String::from_utf16(&pe.szExeFile).unwrap_or_else(|_| String::new());
        if process_name.contains(name) {
            return from_pid(pe.th32ProcessID);
        }

        if unsafe { Process32NextW(handle, &mut pe)} == FALSE {
            break;
        }
    }
    None
}

