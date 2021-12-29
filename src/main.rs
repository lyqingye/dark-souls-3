extern crate winapi;

use crate::game::GameData;
use crate::process::ShareMemMq;
use crate::sync::Mutex;
use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};
use pelite::pe::PeFile;
use pelite::pe64::Pe;
use pelite::Pod;
use std::mem::transmute;

pub mod error;
pub mod game;
pub mod misc;
pub mod overlay;
pub mod pattern;
pub mod process;
pub mod sync;
pub mod window;

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

    // overlay::Overlay::new(0x00040616,game::GameData::init().unwrap())
    //     .unwrap()
    //     .run_loop(|s| {
    //
    //         std::thread::sleep(std::time::Duration::from_millis(1000 / 60));
    //         let refresh_ok = {
    //             s.render_ctx_mut().refresh_world_char_man_data().is_ok()
    //         };
    //         if refresh_ok {
    //             let rect = s.get_rect();
    //             let rt = s.render_ctx();
    //             let chr = rt.world_chr_man();
    //             let players = &chr.session_info_man.players.clone();
    //             let mut offset = 10.0;
    //             for player in players {
    //                 let attrs = player.player_game_data.data.attributes;
    //                 let name = attrs.name_string();
    //                 {
    //                     s.draw_text(
    //                         format!(
    //                             "Name: {}\nSL:   {}\nVig:  {}\nAtt:  {}\nEnd: {}\nVit:   {}\nStr:   {}\nDex: {}\nInt:   {}\nFth:  {}\nLck:  {}\n",
    //                             name,
    //                             attrs.soul_level_string(),
    //                             attrs.vigor_string(),
    //                             attrs.attunement_string(),
    //                             attrs.endurance_string(),
    //                             attrs.vitality_string(),
    //                             attrs.strength_string(),
    //                             attrs.dexterity_string(),
    //                             attrs.intelligence_string(),
    //                             attrs.faith_string(),
    //                             attrs.luck_string(),
    //                         ),
    //                         offset,
    //                         30.0,
    //                         (rect.right - rect.left) as f32,
    //                         (rect.bottom - rect.top) as f32,
    //                     ).unwrap();
    //                 }
    //                 offset += (name.len() + 2 * 16) as f32;
    //             }
    //         }
    //     })
    //     .unwrap();
}
