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
    std::thread::spawn(move || {
        let mut mq = core::sync::Mutex::new("mq",core::process::ShareMemMq::open_or_new("ToolsBox", 1024 * 1024 * 16).unwrap()).unwrap();

        for i in 0..10000000 {
            let mut data = mq.acquire().unwrap();
            let rs = data.enqueue(&[(i as u32) .to_le_bytes().to_vec()]);
            if rs.is_err() {
                println!("{}", rs.err().unwrap());
            } else {
                println!("send success");
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });
}
