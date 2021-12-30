use core::overlay::Overlay;

fn main() {
    let window = core::window::find_window(Some("FDPclass"),None).unwrap();
    Overlay::new(window,core::game::GameData::init().unwrap())
        .unwrap()
        .run_loop(|s| {

            std::thread::sleep(std::time::Duration::from_millis(1000 / 60));
            let refresh_ok = {
                s.render_ctx_mut().refresh_world_char_man_data().is_ok()
            };
            if refresh_ok {
                let rect = s.get_rect();
                let rt = s.render_ctx();
                let chr = rt.world_chr_man();
                let players = &chr.session_info_man.players.clone();
                let mut offset = 10.0;
                for player in players {
                    let attrs = player.player_game_data.data.attributes;
                    let name = attrs.name_string();
                    {
                        s.draw_text(
                            format!(
                                "Name: {}\nSL:   {}\nVig:  {}\nAtt:  {}\nEnd: {}\nVit:   {}\nStr:   {}\nDex: {}\nInt:   {}\nFth:  {}\nLck:  {}\n",
                                name,
                                attrs.soul_level_string(),
                                attrs.vigor_string(),
                                attrs.attunement_string(),
                                attrs.endurance_string(),
                                attrs.vitality_string(),
                                attrs.strength_string(),
                                attrs.dexterity_string(),
                                attrs.intelligence_string(),
                                attrs.faith_string(),
                                attrs.luck_string(),
                            ),
                            offset,
                            30.0,
                            (rect.right - rect.left) as f32,
                            (rect.bottom - rect.top) as f32,
                        ).unwrap();
                    }
                    offset += (name.len() + 2 * 16) as f32;
                }
            }
        })
        .unwrap();
}
