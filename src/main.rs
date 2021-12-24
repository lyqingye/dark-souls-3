extern crate winapi;

use winapi::shared::ntdef::HANDLE;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winnt::{PAGE_EXECUTE_READWRITE, PAGE_READWRITE};

pub mod error;
pub mod overlay;
pub mod process;

fn main() {
    overlay::Overlay::new(0x00040AF2)
        .unwrap()
        .run_loop(&|s| {

        })
        .unwrap();
}
