use crate::utf16_str;
use windows::Win32::Foundation::{HWND, PWSTR};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

pub fn find_window(class_name: Option<&str>, window_name: Option<&str>) -> Option<HWND> {
    let hwnd = unsafe {
        FindWindowW(
            class_name
                .map(|s| utf16_str!(s))
                .unwrap_or(PWSTR(std::ptr::null_mut())),
            window_name
                .map(|s| utf16_str!(s))
                .unwrap_or(PWSTR(std::ptr::null_mut())),
        )
    };
    if hwnd == 0 {
        None
    } else {
        Some(hwnd)
    }
}
