use crate::error::ProcessError;
use crate::process::Process;
use anyhow::{anyhow, Result};

pub fn pattern_search(pattern: String, data: &[u8], find_first: bool) -> Result<Vec<usize>> {
    let mut result = Vec::new();
    if pattern.is_empty() {
        return Ok(result);
    }

    let mut pattern_b: Vec<u8> = Vec::new();
    for code in pattern.split_whitespace().into_iter() {
        if code.len() != 2 {
            return Err(anyhow!("Invalid Pattern"));
        }
        if code == "??" {
            pattern_b.push(b'?');
        } else {
            pattern_b
                .push(u8::from_str_radix(code, 16).map_err(|_| anyhow!("Invalid Hex String"))?);
        }
    }

    let mut wild_addr = 0;
    for i in 0..pattern_b.len() {
        if pattern_b[i] == b'?' {
            wild_addr = i;
        }
    }
    let mut shift: [i32; 256] = [0; 256];
    for i in 0..256 {
        shift[i] = -1;
    }
    for i in wild_addr..pattern_b.len() {
        shift[pattern_b[i] as usize] = (pattern_b.len() - i) as i32;
    }
    for i in 0..256 {
        if shift[i] == -1 {
            shift[i] = (pattern_b.len() - wild_addr) as i32;
        }
    }

    let len_memory = data.len();
    let len_pattern = pattern_b.len();
    let mut i = 0;
    let mut j = 0;
    while i <= (len_memory - len_pattern) {
        while j < len_pattern && (data[i + j] == pattern_b[j] || pattern_b[j] == b'?') {
            j += 1;
        }
        if j == len_pattern {
            result.push(i);
            if find_first {
                return Ok(result);
            }
            i += len_pattern;
        } else {
            if i + len_pattern >= len_memory {
                break;
            }
            i += shift[data[i + len_pattern] as usize] as usize;
        }
        j = 0;
    }
    Ok(result)
}

pub fn remote_pattern_search(
    process: &Process,
    start: usize,
    size: usize,
    page_size: usize,
    pattern: String,
    find_first: bool,
) -> Result<Vec<usize>> {
    let mut result = Vec::new();
    let pattern_size = pattern.len() / 2;
    let page_num;
    let is_align;
    if size % page_size == 0 {
        page_num = size / page_size;
        is_align = true;
    } else {
        page_num = size / page_size + 1;
        is_align = false;
    }
    let mut buffer: Vec<u8> = Vec::with_capacity(page_size + pattern_size);
    for i in 0..page_num {
        let begin = start + page_num * page_size;
        let end;
        if !is_align && i == page_num - 1 {
            end = start + size;
        } else {
            end = begin + page_size;
        }
        let buffer_size = end - begin;
        let rip_base;
        if i == 0 {
            if !process.read_ptr(buffer.as_mut_ptr(), begin, buffer_size) {
                return Err(ProcessError::ReadMemoryFail(begin).into());
            }
            rip_base = begin;
        } else {
            let tail = buffer[buffer_size..].to_vec();
            assert_eq!(tail.len(), pattern_size);
            buffer.fill(0);
            buffer.extend(tail);
            if !process.read_ptr(
                unsafe { buffer.as_mut_ptr().add(pattern_size) },
                begin,
                buffer_size,
            ) {
                return Err(ProcessError::ReadMemoryFail(begin).into());
            }
            rip_base = begin - pattern_size;
        }
        let sub_result = pattern_search(pattern.clone(), buffer.as_slice(), find_first)?;
        for offset in &sub_result {
            result.push(rip_base + offset);
        }
        if find_first && !sub_result.is_empty() {
            assert!(!result.is_empty());
            return Ok(result[0..1].to_vec());
        }
    }
    Ok(result)
}
