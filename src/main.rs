extern crate winapi;

use winapi::shared::ntdef::HANDLE;
pub mod process;
fn main() {
    let ps = process::from_name("notepad.exe").unwrap();
    let value = ps.read::<u64>(0x7FF614B25410).unwrap();
    println!("{:#x}",value);
    println!("{:?}",ps);
}
