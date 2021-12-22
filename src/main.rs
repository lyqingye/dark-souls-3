extern crate winapi;

use winapi::shared::ntdef::HANDLE;
use winapi::um::winnt::{PAGE_EXECUTE_READWRITE, PAGE_READWRITE};
use winapi::um::errhandlingapi::GetLastError;

pub mod process;
fn main() {
    let ps = process::from_name("notepad.exe").unwrap();
    let value = ps.read::<u64>(0x7FF614B25410).unwrap();
    println!("{:#x}",value);
    println!("{:?}",ps);
    let address = ps.alloc(0x64,PAGE_READWRITE).unwrap();
    println!("{:#x}",address);
    println!("{}",ps.protect(address,0x64,PAGE_EXECUTE_READWRITE).unwrap());
    println!("{}",ps.free(address));
}
