use core::overlay::Overlay;
use std::sync::Arc;
#[derive(Debug, Copy, Clone)]
#[repr(packed)]
struct Matrix {
    pub a11: f32,
    pub a12: f32,
    pub a13: f32,
    pub a14: f32,
    pub a21: f32,
    pub a22: f32,
    pub a23: f32,
    pub a24: f32,
    pub a31: f32,
    pub a32: f32,
    pub a33: f32,
    pub a34: f32,
    pub a41: f32,
    pub a42: f32,
    pub a43: f32,
    pub a44: f32,
}

impl Matrix {
    pub fn mul_vec4x1(&self, vec: &Vector4x4) -> Vector4x4 {
        Vector4x4 {
            a1: self.a11 * vec.a1 + self.a12 * vec.a2 + self.a13 * vec.a3 + self.a14 * vec.a4,
            a2: self.a21 * vec.a1 + self.a22 * vec.a2 + self.a23 * vec.a3 + self.a24 * vec.a4,
            a3: self.a31 * vec.a1 + self.a32 * vec.a2 + self.a33 * vec.a3 + self.a34 * vec.a4,
            a4: self.a41 * vec.a1 + self.a42 * vec.a2 + self.a43 * vec.a3 + self.a44 * vec.a4,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(packed)]
struct Vector4x4 {
    pub a1: f32,
    pub a2: f32,
    pub a3: f32,
    pub a4: f32,
}
#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct TypeDescriptor {
    pvftable: usize,
    spare: usize,
    name: char,
}

fn to_ptr<T>(address: usize) -> *const T {
    unsafe { std::ptr::null::<u8>().add(address) as *const T }
}

fn main() {
    let now = std::time::Instant::now();
    let ps = core::process::Process::from_name("DarkSoulsIII.exe").unwrap();
    let xxx = ps.fast_rtti_dump("DarkSoulsIII.exe").unwrap();
    for el in &xxx {
        println!("{:?}", el);
    }
    // let module = ps.get_module("DarkSoulsIII.exe").unwrap();
    // let mut image_buffer = Vec::with_capacity(module.size);
    // image_buffer.resize(module.size, 0);
    // ps.read_ptr(image_buffer.as_mut_ptr(), module.base, module.size);
    // let search_result = core::pattern::pattern_search(
    //     "2E 3F 41 56 74 79 70 65 5F 69 6E 66 6F 40 40".to_owned(),
    //     image_buffer.as_slice(),
    //     false,
    //     Some(module.base),
    // )
    // .unwrap();
    // let desc = ps
    //     .read::<TypeDescriptor>(*search_result.first().unwrap() - 0x10)
    //     .unwrap();
    // let mut p = String::new();
    // let bytes = desc.pvftable.to_le_bytes();
    // for i in 0..bytes.len() {
    //     let c = format!("{:02X}", bytes[i]);
    //     p.push_str(&c);
    //     if i != bytes.len() - 1 {
    //         p.push_str(&*" ".to_string());
    //     }
    // }
    // let mut search_result2 =
    //     core::pattern::pattern_search(p, image_buffer.as_slice(), false, Some(module.base))
    //         .unwrap();
    // search_result2.sort();
    // search_result2.dedup();
    //
    // let pool = threadpool::ThreadPool::new(24);
    // let buffer_arc = Arc::new(image_buffer);
    // let mut index: usize = 1;
    // let counter = search_result2.len();
    // for _type in search_result2 {
    //     let buffer = buffer_arc.clone();
    //     pool.execute(move || {
    //         let ps = core::process::Process::from_name("DarkSoulsIII.exe").unwrap();
    //         let type_offset = (_type - module.base) as u32;
    //         let bytes = type_offset.to_le_bytes();
    //         if let Ok(mut references) =
    //             core::pattern::pattern_search2(&bytes, buffer.as_slice(), false, Some(module.base))
    //         {
    //             references.sort();
    //             references.dedup();
    //             for reference in references {
    //                 if let Ok(mem_info) = ps.query_memory_info(reference) {
    //                     // code address
    //                     if mem_info.Protect & 0x10 == 0x10 {
    //                         continue;
    //                     }
    //                     if let Ok(0) = ps.read::<u64>(_type) {
    //                         continue;
    //                     }
    //                     let object_locator: usize = reference - 0xc;
    //                     let object_locator_bytes = object_locator.to_le_bytes();
    //
    //                     if let Ok(mut meta_pointers) = core::pattern::pattern_search2(
    //                         &object_locator_bytes,
    //                         buffer.as_slice(),
    //                         true,
    //                         Some(module.base),
    //                     ) {
    //                         meta_pointers.sort();
    //                         meta_pointers.dedup();
    //                         if meta_pointers.len() == 1 {
    //                             let meta = meta_pointers.first().unwrap();
    //                             let vft = *meta + 0x8;
    //                             // let vft_rva = vft - module.base;
    //
    //                             let mut class_name = "".to_string();
    //                             if let Ok(type_desc) = ps.read_utf8_str(_type + 0x10, 255, &['@']) {
    //                                 class_name = type_desc;
    //                             }
    //
    //                             if let Ok(heirarchy_offset) = ps.read::<u32>(object_locator + 0x10)
    //                             {
    //                                 let class_heirarchy = module.base + heirarchy_offset as usize;
    //                                 if let Ok(class_cnt) = ps.read::<u32>(class_heirarchy + 0x8) {
    //                                     if class_cnt > 255 {
    //                                         println!(
    //                                             "{:x} {:x} {:x } {:x} {:x}",
    //                                             _type, vft, object_locator, class_heirarchy, *meta
    //                                         );
    //                                         continue;
    //                                     }
    //                                     if let Ok(base_class_offset) =
    //                                         ps.read::<u32>(class_heirarchy + 0xc)
    //                                     {
    //                                         let class_array =
    //                                             base_class_offset as usize + module.base;
    //                                         for i in 1..class_cnt {
    //                                             if let Ok(td_offset) =
    //                                                 ps.read::<u32>((i * 4) as usize + class_array)
    //                                             {
    //                                                 if let Ok(td_ptr) = ps.read::<u32>(
    //                                                     td_offset as usize + module.base,
    //                                                 ) {
    //                                                     let td = td_ptr as usize + module.base;
    //
    //                                                     if let Ok(type_desc) =
    //                                                         ps.read_utf8_str(td + 0x10, 255, &[])
    //                                                     {
    //                                                         class_name = format!(
    //                                                             "{}::{}",
    //                                                             class_name, type_desc
    //                                                         );
    //                                                     }
    //                                                 }
    //                                             }
    //                                         }
    //                                     }
    //                                 }
    //                             }
    //                             // println!("{:X} {}",vft ,class_name);
    //                         }
    //                     }
    //                 }
    //             }
    //
    //             // println!("{}/{}",index,counter);
    //         }
    //     });
    //     index += 1;
    // }
    // while pool.active_count() > 0 {
    //     // println!("{}",pool.active_count());
    //     std::thread::sleep_ms(1);
    // }
    println!("{}ms", now.elapsed().as_millis());
    // println!("{:?}",desc);

    // println!("{:?}", ps.get_rtti(0x0007FF4AD045B70).unwrap());
    // if let Ok(rtti) = ps.fast_rtti_dump(0x7000000000000000, 0x7fffffffffffffff) {
    //     println!("{:?}", rtti);

    // let mut window_some = core::window::find_window(Some("FDPclass"), Some("DARK SOULS III"));
    // while window_some == None {
    //     window_some = core::window::find_window(Some("FDPclass"), Some("DARK SOULS III"));
    // }
    // let window = window_some.unwrap();
    //
    // Overlay::new(window,core::game::GameData::init().unwrap())
    //     .unwrap()
    //     .run_loop(|s| {
    //         // std::thread::sleep(std::time::Duration::from_millis(1000 / 60));
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
    //
    //             {
    //                 let rt = s.render_ctx();
    //                 let matrix = rt.ps.read::<Matrix>(0x00007FF4ADF19D10).unwrap();
    //                 let pos = rt.ps.read::<Vector4x4>(0x00007FF4AD047010).unwrap();
    //                 let mut screen_vec = matrix.mul_vec4x1(&pos);
    //                 println!("pos: {:?}", screen_vec.clone());
    //                 s.draw_text("person".to_string(),
    //                             screen_vec.a1.abs() + 250.0 ,
    //                             screen_vec.a2.abs() + 300.0,
    //                             (rect.right - rect.left) as f32,
    //                             (rect.bottom - rect.top) as f32,
    //                 );
    //             }
    //
    //         }
    //     })
    //     .unwrap();
}
