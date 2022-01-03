#![cfg(windows)]

use std::sync::Arc;
use winapi::shared::minwindef;
use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID};
use winapi::um::consoleapi;

#[no_mangle]
#[allow(non_snake_case, unused_variables)]
extern "system" fn DllMain(dll_module: HINSTANCE, call_reason: DWORD, reserved: LPVOID) -> BOOL {
    const DLL_PROCESS_ATTACH: DWORD = 1;
    const DLL_PROCESS_DETACH: DWORD = 0;

    match call_reason {
        DLL_PROCESS_ATTACH => main(),
        DLL_PROCESS_DETACH => (),
        _ => (),
    }
    minwindef::TRUE
}

fn main() {
    unsafe { consoleapi::AllocConsole() };
    let buffer = Arc::new([0u8; 125]);

    for i in 0..12 {
        let ptr = buffer.clone();
        std::thread::spawn(move || {
            ptr.get(0);
            println!("{}", ptr[0]);
        });
    }
}
