extern crate winapi;

pub mod error;
pub mod overlay;
pub mod process;
pub mod game;

fn main() {
    let mut game_data = game::GameData::init().unwrap();
    game_data.refresh_world_char_man_data().unwrap();
    println!("{:#?}",game_data);
    overlay::Overlay::new(0x001506C6)
        .unwrap()
        .run_loop(|s| {
            s.draw_text("hello".to_owned(),0.0,0.0);
        })
        .unwrap();
}
