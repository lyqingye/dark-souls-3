use crate::error::ProcessError;
use crate::process::Process;
use anyhow::{anyhow, Result};
use std::ops::IndexMut;

pub fn pattern_search(
    pattern: String,
    data: &[u8],
    find_first: bool,
    base: Option<usize>,
) -> Result<Vec<usize>> {
    let mut result = Vec::new();
    if pattern.is_empty() {
        return Ok(result);
    }

    let mut pattern_b: Vec<u8> = Vec::new();
    let mut wild_addr = 0;
    let mut wild_mask: Vec<bool> = Vec::new();
    let mut index = 0;
    for code in pattern.split_whitespace().into_iter() {
        if code.len() != 2 {
            return Err(anyhow!("Invalid Pattern"));
        }
        if code == "??" {
            pattern_b.push(b'?');
            wild_mask.push(true);
            wild_addr = index;
        } else {
            wild_mask.push(false);
            pattern_b
                .push(u8::from_str_radix(code, 16).map_err(|_| anyhow!("Invalid Hex String"))?);
        }
        index += 1;
    }

    let mut shift: [i32; 256] = [0; 256];
    for i in 0..256 {
        shift[i] = -1;
    }
    for i in (wild_addr + 1)..pattern_b.len() {
        shift[pattern_b[i] as usize] = (pattern_b.len() - i) as i32;
    }
    for i in 0..256 {
        if shift[i] == -1 {
            shift[i] = (pattern_b.len() - wild_addr) as i32;
        }
    }

    let mask_buffer = wild_mask.as_slice();
    let len_memory = data.len();
    let len_pattern = pattern_b.len();
    let mut i = 0;
    let mut j = 0;
    while i <= (len_memory - len_pattern) {
        while j < len_pattern && (data[i + j] == pattern_b[j] || mask_buffer[j] == true) {
            j += 1;
        }
        if j == len_pattern {
            if let Some(rip) = base {
                result.push(rip + i);
            } else {
                result.push(i);
            }
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

pub fn pattern_search2(
    pattern: &[u8],
    data: &[u8],
    find_first: bool,
    base: Option<usize>,
) -> Result<Vec<usize>> {
    let mut p = String::new();
    for i in 0..pattern.len() {
        let c = format!("{:02X}", pattern[i]);
        p.push_str(&c);
        if i != pattern.len() - 1 {
            p.push_str(&*" ".to_string());
        }
    }
    pattern_search(p, data, find_first, base)
}

pub fn remote_pattern_search2(
    process: &Process,
    start: usize,
    size: usize,
    page_size: usize,
    pattern: &[u8],
    find_first: bool,
) -> Result<Vec<usize>> {
    let mut p = String::new();
    for i in 0..pattern.len() {
        let c = format!("{:02X}", pattern[i]);
        p.push_str(&c);
        if i != pattern.len() - 1 {
            p.push_str(&*" ".to_string());
        }
    }
    remote_pattern_search(process, start, size, page_size, p, find_first)
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
    let pattern_size = pattern.replace(" ", "").to_string().len() / 2;
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
    buffer.resize(page_size + pattern_size, 0);
    for i in 0..page_num {
        let begin = start + i * page_size;
        let end;
        if !is_align && i == page_num - 1 {
            end = start + size;
        } else {
            end = begin + page_size;
        }
        let buffer_size = end - begin;
        let rip_base;
        let success;
        if i == 0 {
            success = process.read_ptr(buffer.as_mut_ptr(), begin, buffer_size);
            rip_base = begin;
        } else {
            let tail = buffer[page_size..].to_vec();
            assert_eq!(tail.len(), pattern_size);
            buffer.fill(0);
            for i in 0..pattern_size {
                *buffer.get_mut(0).unwrap() = *tail.get(i).unwrap();
            }
            success = process.read_ptr(
                unsafe { buffer.as_mut_ptr().add(pattern_size) },
                begin,
                buffer_size,
            );
            rip_base = begin - pattern_size;
        }
        if success.is_err() {
            continue;
        }
        let sub_result = pattern_search(pattern.clone(), buffer.as_slice(), find_first, None)?;
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
