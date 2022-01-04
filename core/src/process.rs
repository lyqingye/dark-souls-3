use crate::error::ProcessError;
use crate::sync::Mutex;

use anyhow::{anyhow, Result};

use crossbeam_channel::{unbounded, Sender};
use std::borrow::BorrowMut;
use std::cmp::max;
use std::collections::HashSet;
use std::fs::read;
use std::ops::DerefMut;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::{mem, ptr};
use winapi::shared::basetsd::SIZE_T;
use winapi::shared::minwindef::{BOOL, DWORD, FALSE, LPCVOID, LPVOID, PBOOL, PDWORD};
use winapi::shared::ntdef::HANDLE;

use crate::error::ProcessError::ProcessNotFound;
use crate::pattern::{pattern_search2, remote_pattern_search, remote_pattern_search2};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::memoryapi::{
    CreateFileMappingW, MapViewOfFile, OpenFileMappingW, ReadProcessMemory, UnmapViewOfFile,
    VirtualAllocEx, VirtualFreeEx, VirtualProtectEx, VirtualQuery, WriteProcessMemory,
    FILE_MAP_ALL_ACCESS,
};
use winapi::um::processthreadsapi::{GetCurrentProcessId, OpenProcess};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Process32FirstW, Process32NextW,
    MODULEENTRY32W, PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use winapi::um::winnt::{
    MEMORY_BASIC_INFORMATION, MEMORY_BASIC_INFORMATION64, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
    PAGE_READWRITE, PMEMORY_BASIC_INFORMATION, PMEMORY_BASIC_INFORMATION64, PROCESS_ALL_ACCESS,
};
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

#[derive(Debug, Clone, PartialEq)]
pub struct RTTIInfo {
    pub vf_ptr: usize,
    pub vf_meta: usize,
    pub type_desc: String,
    pub base_class: Vec<String>,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct TypeDescriptor {
    pvftable: usize,
    spare: usize,
    name: char,
}

impl Process {
    pub fn current_process() -> Option<Process> {
        unsafe { Process::from_pid(GetCurrentProcessId()) }
    }
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

    pub fn read_ptr<T: Copy>(&self, buf: *mut T, address: usize, count: usize) -> Result<()> {
        unsafe {
            if ReadProcessMemory(
                self.handle,
                address as LPCVOID,
                buf as LPVOID,
                mem::size_of::<T>() as SIZE_T * count,
                ptr::null_mut::<SIZE_T>(),
            ) != FALSE
            {
                Ok(())
            } else {
                Err(ProcessError::ReadMemoryFail(address).into())
            }
        }
    }

    pub fn read_utf8_str(
        &self,
        address: usize,
        max_length: usize,
        filter: &[char],
    ) -> Result<String> {
        let mut buf: Vec<u8> = Vec::with_capacity(max_length);
        buf.resize(max_length, 0);
        self.read_ptr(buf.as_mut_ptr(), address, max_length)?;
        let mut str = String::with_capacity(max_length);
        'label: for b in buf {
            let c = b as char;
            if c == '\0' {
                break;
            }
            for f in filter {
                if c == *f {
                    break 'label;
                }
            }
            // if c.is_ascii_graphic() || c.is_ascii_digit() {
            //     str.push(c);
            // }
            str.push(c);
        }
        Ok(str)
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

    pub fn query_memory_info(&self, address: usize) -> Result<MEMORY_BASIC_INFORMATION> {
        unsafe {
            let mut information = mem::zeroed::<MEMORY_BASIC_INFORMATION>();
            if VirtualQuery(
                address as LPCVOID,
                &mut information,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>() as SIZE_T,
            ) != 0
            {
                Ok(information)
            } else {
                Err(ProcessError::QueryMemoryFail(address).into())
            }
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

    pub fn fast_rtti_dump(&self, module: &str) -> Result<Vec<RTTIInfo>> {
        let pool = threadpool::ThreadPool::new(num_cpus::get());
        let module = self
            .get_module(module)
            .ok_or(ProcessError::ModuleNotFound)?;
        let mut img_buf = Vec::with_capacity(module.size);
        img_buf.resize(module.size, 0);
        self.read_ptr(img_buf.as_mut_ptr(), module.base, module.size)?;
        let read_only_buffer = Arc::new(img_buf);
        let sign: Vec<usize> = pattern_search2(
            b".?AVtype_info@@",
            read_only_buffer.clone().as_slice(),
            false,
            Some(module.base),
        )?;
        if let Some(sign) = sign.first() {
            let type_desc: TypeDescriptor = self.read::<TypeDescriptor>(*sign - 0x10)?;
            let mut types: Vec<usize> = pattern_search2(
                &type_desc.pvftable.to_le_bytes(),
                read_only_buffer.clone().as_slice(),
                false,
                Some(module.base),
            )?;
            types.sort();
            types.dedup();
            let (_tx, rx) = unbounded::<RTTIInfo>();
            for _type in types {
                let buffer = read_only_buffer.clone();
                let pid = self.id;
                let tx = _tx.clone();
                pool.execute(move || {
                    let ps = Process::from_pid(pid).unwrap();
                    let type_offset = (_type - module.base) as u32;
                    if let Ok(mut references) = pattern_search2(
                        &type_offset.to_le_bytes(),
                        buffer.as_slice(),
                        false,
                        Some(module.base),
                    ) {
                        references.sort();
                        references.dedup();
                        for reference in references {
                            if let Ok(0) = ps.read::<usize>(_type) {
                                continue;
                            }
                            let object_locator: usize = reference - 0xc;
                            if let Ok(mut meta_pointers) = pattern_search2(
                                &object_locator.to_le_bytes(),
                                buffer.as_slice(),
                                true,
                                Some(module.base),
                            ) {
                                meta_pointers.sort();
                                meta_pointers.dedup();
                                if meta_pointers.len() == 1 {
                                    let meta = meta_pointers.first().unwrap();
                                    if let Ok(mut rtti) = Self::get_rtti_from_type(
                                        &ps,
                                        _type,
                                        object_locator,
                                        module.base,
                                    ) {
                                        rtti.vf_ptr = *meta + 0x8;
                                        rtti.vf_meta = *meta;
                                        tx.send(rtti);
                                    }
                                }
                            }
                        }
                    }
                });
            }

            let mut result = Vec::with_capacity(1024);
            let mut set = HashSet::with_capacity(1024);
            while pool.active_count() > 0 {
                if let Ok(rtti) = rx.recv() {
                    if !set.contains(&rtti.vf_ptr) {
                        set.insert(rtti.vf_ptr);
                        result.push(rtti);
                    }
                }
            }
            while let Ok(rtti) = rx.try_recv() {
                if !set.contains(&rtti.vf_ptr) {
                    set.insert(rtti.vf_ptr);
                    result.push(rtti);
                }
            }
            return Ok(result);
        }

        Ok(Vec::new())
    }

    fn get_rtti_from_type(
        ps: &Process,
        _type: usize,
        object_locator: usize,
        base: usize,
    ) -> Result<RTTIInfo> {
        let class_name = ps.read_utf8_str(_type + 0x10, 255, &[])?;
        let class_heirarchy = ps.read::<u32>(object_locator + 0x10)? as usize + base;
        let class_cnt = ps.read::<u32>(class_heirarchy + 0x8)?;
        let class_array = ps.read::<u32>(class_heirarchy + 0xc)? as usize + base;
        let mut base_class = Vec::new();
        for i in 0..class_cnt {
            let td_offset = ps.read::<u32>((i * 4) as usize + class_array)?;
            let td = ps.read::<u32>(td_offset as usize + base)? as usize + base;
            base_class.push(ps.read_utf8_str(td + 0x10, 255, &[])?);
        }
        Ok(RTTIInfo {
            type_desc: class_name,
            vf_ptr: 0,
            vf_meta: 0,
            base_class,
        })
    }

    pub fn pattern_search(
        &self,
        start: usize,
        size: usize,
        pattern: String,
        find_first: bool,
    ) -> Result<Vec<usize>> {
        remote_pattern_search(self, start, size, size, pattern, find_first)
    }

    pub fn pattern_search2(
        &self,
        start: usize,
        size: usize,
        pattern: &[u8],
        find_first: bool,
    ) -> Result<Vec<usize>> {
        remote_pattern_search2(self, start, size, size, pattern, find_first)
    }

    pub fn pattern_search3<T: Sized>(
        &self,
        start: usize,
        size: usize,
        pattern: &T,
        find_first: bool,
    ) -> Result<Vec<usize>> {
        unsafe {
            let buffer = std::ptr::slice_from_raw_parts(
                pattern as *const T as *const u8,
                std::mem::size_of::<T>(),
            );
            self.pattern_search2(start, size, &*buffer, find_first)
        }
    }

    pub fn open_shmemq(&self, _name: &str, _create: bool, _size: usize) -> Result<()> {
        Ok(())
    }
}

fn safe_add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).ok_or(anyhow!("Overflow"))
}

fn safe_sub(a: usize, b: usize) -> Result<usize> {
    a.checked_sub(b).ok_or(anyhow!("Overflow"))
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
    data: &'a mut [u8],

    // Handles
    map: HANDLE,
    buf: LPVOID,
}

#[repr(packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ShareMemMqMeta {
    size: usize,
    r_index: usize,
    w_index: usize,
    head_size: usize,
}

impl<'a> ShareMemMq<'a> {
    pub fn open_or_new(name: &str, size: usize) -> Result<Self> {
        let mut lock = Mutex::acquire(name)?;
        let full_name = format!("{}", name);
        let buf_size = std::mem::size_of::<ShareMemMqMeta>() + size;
        let is_new;
        let file = unsafe {
            let test_handle =
                OpenFileMappingW(FILE_MAP_ALL_ACCESS, FALSE, full_name.as_ptr() as *mut u16);
            if test_handle.is_null() {
                is_new = true;
                CreateFileMappingW(
                    INVALID_HANDLE_VALUE,
                    std::ptr::null_mut(),
                    PAGE_READWRITE,
                    0,
                    buf_size as DWORD,
                    full_name.as_ptr() as *mut u16,
                )
            } else {
                is_new = false;
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
        if is_new {
            meta.size = buf_size;
            meta.head_size = std::mem::size_of::<ShareMemMqMeta>();
            meta.r_index = 0;
            meta.w_index = 0;
        }
        let data_ptr = unsafe { buf.cast::<u8>().add(std::mem::size_of::<ShareMemMqMeta>()) };
        let data = unsafe {
            &mut *std::ptr::slice_from_raw_parts_mut(&mut *data_ptr, meta.size - meta.head_size)
        };
        lock.release();
        Ok(Self {
            name: full_name.clone(),
            meta,
            map: file,
            buf,
            data,
        })
    }

    pub fn peek(&self) -> Option<&[u8]> {
        let mut lock = Mutex::acquire(self.name.as_str()).ok()?;
        let data_size = self.meta.size - self.meta.head_size;
        let remain_read_bytes = self.meta.w_index - self.meta.r_index;
        if self.meta.r_index == self.meta.w_index || remain_read_bytes <= std::mem::size_of::<u16>()
        {
            lock.release();
            None
        } else {
            unsafe {
                let ptr_to_el = self.data.as_ptr().add(self.meta.r_index % data_size) as *const u16;
                let el_size = ptr_to_el.read();
                if (el_size as usize) > remain_read_bytes {
                    lock.release();
                    None
                } else {
                    let ptr = self
                        .data
                        .as_ptr()
                        .add((self.meta.r_index % data_size) + std::mem::size_of::<u16>())
                        as *mut u8;
                    lock.release();
                    Some(&mut *std::ptr::slice_from_raw_parts_mut(
                        ptr,
                        el_size as usize - std::mem::size_of::<u16>(),
                    ))
                }
            }
        }
    }

    pub fn dequeue(&mut self) -> Result<Vec<Vec<u8>>> {
        let mut lock = Mutex::acquire(self.name.as_str())?;
        let data_size = self.meta.size - self.meta.head_size;
        let mut remain_read_bytes = self.meta.w_index - self.meta.r_index;
        let mut result = Vec::new();
        while self.meta.r_index < self.meta.w_index
            && remain_read_bytes > std::mem::size_of::<u16>()
        {
            unsafe {
                let ptr_to_el = self.data.as_ptr().add(self.meta.r_index % data_size) as *const u16;
                let el_size = ptr_to_el.read();
                if (el_size as usize) > remain_read_bytes
                    || (el_size as usize <= std::mem::size_of::<u16>())
                {
                    return Err(ProcessError::InvalidShareMemMq.into());
                } else {
                    let ptr = self
                        .data
                        .as_ptr()
                        .add((self.meta.r_index % data_size) + std::mem::size_of::<u16>())
                        as *mut u8;
                    let el = &*std::ptr::slice_from_raw_parts_mut(
                        ptr,
                        el_size as usize - std::mem::size_of::<u16>(),
                    );
                    self.meta.r_index = self.meta.r_index + el_size as usize;
                    result.push(el.to_vec());
                }
            }
            remain_read_bytes = self.meta.w_index - self.meta.r_index;
        }
        lock.release();
        Ok(result)
    }

    pub fn enqueue(&mut self, elements: &[Vec<u8>]) -> Result<()> {
        let mut lock = Mutex::acquire(self.name.as_str())?;
        let data_size = self.meta.size - self.meta.head_size;
        let remain_bytes = self.meta.size - (self.meta.w_index - self.meta.r_index);
        let mut size_of = 0;
        for el in elements {
            size_of += el.len() + std::mem::size_of::<u16>();
        }
        if size_of > remain_bytes {
            return Err(ProcessError::ShareMemMqHasFull.into());
        }
        for el in elements {
            let idx = self.meta.w_index % data_size;
            unsafe {
                let ptr_to_size = self.data.as_ptr().add(idx) as *mut u16;
                ptr_to_size.write((el.len() + std::mem::size_of::<u16>()) as u16);
                let ptr_to_el = self.data.as_ptr().add(idx + std::mem::size_of::<u16>()) as *mut u8;
                let new_el = &mut *std::ptr::slice_from_raw_parts_mut(ptr_to_el, el.len() as usize);
                new_el.copy_from_slice(el.as_slice());
                self.meta.w_index = self.meta.w_index + el.len() + std::mem::size_of::<u16>();
            }
        }
        lock.release();
        Ok(())
    }
}

impl<'a> Drop for ShareMemMq<'a> {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(self.buf as LPCVOID);
            CloseHandle(self.map);
        };
    }
}

mod test {
    use crate::process::{ShareMemMq, ShareMemMqMeta};

    #[test]
    pub fn test_share_memory_queue() {
        let mut mq = ShareMemMq::open_or_new("test_memory_queue", 4890).unwrap();
        assert_eq!(4890 + std::mem::size_of::<ShareMemMqMeta>(), mq.meta.size);
        assert_eq!(0, mq.meta.r_index);
        assert_eq!(0, mq.meta.w_index);
        assert_eq!(None, mq.peek());

        let mut data_list = Vec::new();
        let mut el_sizes = 0;
        for i in 0..1000 {
            let bytes = i.to_string().into_bytes();
            el_sizes += bytes.len() + std::mem::size_of::<u16>();
            data_list.push(bytes);
        }
        mq.enqueue(data_list.as_slice()).unwrap();

        assert_eq!(0, mq.meta.r_index);
        assert_eq!(el_sizes, mq.meta.w_index);
        assert_eq!(el_sizes, mq.meta.w_index - mq.meta.r_index);

        let dequeue_data_list = mq.dequeue().unwrap();
        assert_eq!(data_list, dequeue_data_list);
        assert_eq!(mq.meta.r_index, el_sizes);
        assert_eq!(mq.meta.w_index, el_sizes);
        assert_eq!(None, mq.peek());
    }

    #[test]
    pub fn test_share_memory_queue2() {
        let mut mq = ShareMemMq::open_or_new("test_memory_queue", 4890).unwrap();
        let mut data_list = Vec::new();
        let mut el_sizes = 0;
        for i in 0..500 {
            let bytes = i.to_string().into_bytes();
            el_sizes += bytes.len() + std::mem::size_of::<u16>();
            data_list.push(bytes);
        }
        mq.enqueue(data_list.as_slice()).unwrap();
        assert_eq!(mq.dequeue().unwrap(), data_list);

        let mut data_list2 = Vec::new();
        let mut el_sizes2 = 0;
        for i in 0..1000 {
            let bytes = i.to_string().into_bytes();
            el_sizes2 += bytes.len() + std::mem::size_of::<u16>();
            data_list2.push(bytes);
        }
        mq.enqueue(data_list2.as_slice()).unwrap();
        assert_eq!(mq.dequeue().unwrap(), data_list2);
    }
}
