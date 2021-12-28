#[macro_export]
macro_rules! utf8_str {
    ($str: expr) => {
        windows::Win32::Foundation::PSTR(format!("{}\0", $str).as_ptr() as _)
    };
}

#[macro_export]
macro_rules! utf16_str {
    ($str: expr) => {
        windows::Win32::Foundation::PWSTR($str.encode_utf16().collect::<Vec<u16>>().as_mut_ptr())
    };
}
