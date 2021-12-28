use crate::error::ProcessError;
use crate::utf16_str;
use anyhow::Result;
use std::ops::{Deref, DerefMut};
use std::{mem, ptr};
use winapi::shared::basetsd::SIZE_T;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, LPCVOID, LPVOID, PBOOL, PDWORD, TRUE};
use winapi::shared::ntdef::HANDLE;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::memoryapi::{
    CreateFileMappingW, FlushViewOfFile, MapViewOfFile, OpenFileMappingW, ReadProcessMemory,
    UnmapViewOfFile, VirtualAllocEx, VirtualFreeEx, VirtualProtectEx, WriteProcessMemory,
    FILE_MAP_ALL_ACCESS,
};
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use winapi::um::winnt::{MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, PROCESS_ALL_ACCESS};
use winapi::um::wow64apiset::IsWow64Process;

#[derive(Debug, Clone)]
pub struct Process {
    pub id: u32,
    pub is_wow64: bool,
    handle: HANDLE,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub base: usize,
    pub size: usize,
}

impl Process {
    pub fn from_pid(pid: u32) -> Option<Process> {
        let handle = unsafe { OpenProcess(PROCESS_ALL_ACCESS, 0, pid) };
        if handle.is_null() {
            return None;
        }

        let mut tmp: BOOL = 0;
        if unsafe { IsWow64Process(handle, &mut tmp as PBOOL) } == FALSE {
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
        if unsafe { Process32FirstW(handle, &mut pe) } == FALSE {
            return None;
        }

        loop {
            let process_name = String::from_utf16(&pe.szExeFile).unwrap_or_else(|_| String::new());
            if process_name.contains(name) {
                return Self::from_pid(pe.th32ProcessID);
            }

            if unsafe { Process32NextW(handle, &mut pe) } == FALSE {
                break;
            }
        }
        None
    }

    pub fn read<T: Copy>(&self, address: usize) -> Result<T> {
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
            FALSE => Err(ProcessError::ReadMemoryFail(address).into()),
            _ => Ok(buffer),
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

    pub fn write<T: Copy>(&self, address: usize, buf: &T) -> bool {
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

    pub fn alloc(&self, size: usize, protection: DWORD) -> Option<usize> {
        let buffer = unsafe {
            VirtualAllocEx(
                self.handle,
                ptr::null_mut(),
                size,
                MEM_RESERVE | MEM_COMMIT,
                protection,
            )
        };
        return if buffer.is_null() {
            None
        } else {
            Some(buffer as usize)
        };
    }

    pub fn free(&self, address: usize) -> bool {
        match unsafe { VirtualFreeEx(self.handle, address as LPVOID, 0 as SIZE_T, MEM_RELEASE) } {
            FALSE => false,
            _ => true,
        }
    }

    pub fn protect(&self, address: usize, size: usize, protection: DWORD) -> Option<DWORD> {
        let mut tmp: DWORD = 0;
        match unsafe {
            VirtualProtectEx(
                self.handle,
                address as LPVOID,
                size,
                protection,
                &mut tmp as PDWORD,
            )
        } {
            FALSE => None,
            _ => Some(tmp),
        }
    }

    pub fn get_module(&self, name: &str) -> Option<Module> {
        let handle =
            unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, self.id) };

        if handle.is_null() {
            return None;
        }
        let mut me: MODULEENTRY32W = unsafe { mem::zeroed() };
        me.dwSize = mem::size_of::<MODULEENTRY32W>() as u32;

        if unsafe { Module32FirstW(handle, &mut me) } == FALSE {
            return None;
        }

        loop {
            let s = String::from_utf16_lossy(&me.szModule)
                .trim_matches('\0')
                .to_string();
            if name == s {
                return Some(Module {
                    name: s,
                    base: me.modBaseAddr as usize,
                    size: me.modBaseSize as usize,
                });
            }

            if unsafe { Module32NextW(handle, &mut me) } == FALSE {
                break;
            }
        }
        None
    }

    pub fn open_shmemq(&self, name: &str, create: bool, size: usize) -> Result<()> {
        Ok(())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { CloseHandle(self.handle) };
        }
    }
}

#[derive(Debug)]
pub struct ShareMemMq<'a> {
    name: String,
    meta: &'a mut ShareMemMqMeta,
    buf: LPVOID,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ShareMemMqMeta {
    el_size: usize,
    size: usize,
    r_index: usize,
    w_index: usize,
    head_size: usize,
}

impl<'a> ShareMemMq<'a> {
    pub fn new<T>(name: &str, size: usize) -> Result<Self> {
        let full_name = format!("Global\\{}", name);
        let buf_size = std::mem::size_of::<ShareMemMqMeta>() + size * std::mem::size_of::<T>();
        let file = unsafe {
            let test_handle =
                OpenFileMappingW(FILE_MAP_ALL_ACCESS, FALSE, full_name.as_ptr() as *mut u16);
            if test_handle.is_null() {
                println!("{:#x}", unsafe { GetLastError() });
                CreateFileMappingW(
                    INVALID_HANDLE_VALUE,
                    std::ptr::null_mut(),
                    PAGE_READWRITE,
                    0,
                    buf_size as DWORD,
                    full_name.as_ptr() as *mut u16,
                )
            } else {
                test_handle
            }
        };
        if file.is_null() {
            return Err(ProcessError::CreateFileMapping(full_name.clone()).into());
        }

        let buf = unsafe { MapViewOfFile(file, FILE_MAP_ALL_ACCESS, 0, 0, buf_size as SIZE_T) };

        if buf.is_null() {
            unsafe { CloseHandle(file) };
            return Err(ProcessError::CreateFileMapping(full_name.clone()).into());
        }

        let meta = unsafe { &mut *buf.cast::<ShareMemMqMeta>() };
        meta.el_size = std::mem::size_of::<T>();
        meta.size = size;
        meta.r_index = 0;
        meta.w_index = 0;
        meta.head_size = std::mem::size_of::<ShareMemMqMeta>();

        Ok(Self {
            name: full_name.clone(),
            meta,
            buf,
        })
    }
}

impl<'a> Drop for ShareMemMq<'a> {
    fn drop(&mut self) {
        unsafe {
            assert_eq!(
                TRUE,
                FlushViewOfFile(
                    self.buf as LPCVOID,
                    self.meta.head_size + self.meta.size * self.meta.el_size
                )
            );
            assert_eq!(TRUE, UnmapViewOfFile(self.buf as LPCVOID));
        };
    }
}

mod test {
    use crate::process::ShareMemMq;

    #[test]
    pub fn test_shmemq() {
        let mq = ShareMemMq::new::<u8>("myfile", 1024).unwrap();
        mq.meta.w_index = mq.meta.w_index + 1;
        println!("{:?}", mq)
    }
}
