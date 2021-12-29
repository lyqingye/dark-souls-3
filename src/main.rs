


fn main() {

    // let file_map = pelite::FileMap::open(
    //     "C:\\Program Files (x86)\\Steam\\steamapps\\common\\DARK SOULS III\\Game\\DarkSoulsIII.exe",
    // )
    // .expect("file not found");
    // let file_bytes = file_map.as_ref();
    // let pe = PeFile::from_bytes(file_bytes).expect("invalid pe file");
    // let text_sec = pe
    //     .section_headers()
    //     .by_name(".text")
    //     .expect("text section not found");
    //
    // let RIP = (pe.nt_headers().OptionalHeader.ImageBase + text_sec.VirtualAddress as u64) as u64;
    // let ptr = unsafe { file_bytes.as_ptr().add(text_sec.PointerToRawData as usize) };
    // let text_sec_start = ptr.as_bytes();
    // let mut decoder = Decoder::with_ip(64, text_sec_start, RIP, DecoderOptions::NONE);
    // let mut formatter = NasmFormatter::new();
    // formatter.options_mut().set_digit_separator("`");
    // formatter.options_mut().set_first_operand_char_index(10);
    // // String implements FormatterOutput
    // let mut output = String::new();
    //
    // // Initialize this outside the loop because decode_out() writes to every field
    // let mut instruction = Instruction::default();
    // while decoder.can_decode() {
    //     decoder.decode_out(&mut instruction);
    //
    //     // Format the instruction ("disassemble" it)
    //     output.clear();
    //     formatter.format(&instruction, &mut output);
    //
    //     print!("{:016X} ", instruction.ip());
    //     let start_index = (instruction.ip() - RIP) as usize;
    //     let instr_bytes = &text_sec_start[start_index..start_index + instruction.len()];
    //     for b in instr_bytes.iter() {
    //         print!("{:02X}", b);
    //     }
    //     if instr_bytes.len() < 10 {
    //         for _ in 0..16 - instr_bytes.len() {
    //             print!("  ");
    //         }
    //     }
    //     println!(" {}", output);
    // }


}
