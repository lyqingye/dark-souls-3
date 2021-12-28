use crate::error::ProcessError;
use crate::process::Process;
use anyhow::Result;

const PROCESS_NAME: &'static str = "DarkSoulsIII.exe";
// const PROCESS_NAME: &'static str = "notepad.exe";

#[derive(Debug, Clone)]
pub struct GameData {
    ps: Process,
    world_chr_man: WorldChrMan,
}

impl GameData {
    pub fn init() -> Result<GameData> {
        let process = Process::from_name(PROCESS_NAME)
            .ok_or(ProcessError::ProcessNotFound(PROCESS_NAME.to_string()))?;
        let world_chr_man = WorldChrMan::init(&process)?;
        Ok(Self {
            ps: process,
            world_chr_man,
        })
    }

    pub fn refresh_world_char_man_data(&mut self) -> Result<()> {
        self.world_chr_man.refresh_data(&self.ps)
    }

    pub fn world_chr_man(&self) -> &WorldChrMan {
        &self.world_chr_man
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorldChrMan {
    image_base: usize,
    world_char_man: usize,

    // Data
    pub player_ins: PlayerIns,
    pub session_info_man: SessionInfoMan,
}

impl WorldChrMan {
    pub fn init(ps: &Process) -> Result<WorldChrMan> {
        let mut man = WorldChrMan::default();
        man.image_base = ps
            .get_module(PROCESS_NAME)
            .ok_or(ProcessError::ModuleNotFound)?
            .base;
        Ok(man)
    }

    pub fn refresh_data(&mut self, ps: &Process) -> Result<()> {
        self.world_char_man = ps.read::<usize>(self.image_base + 0x4768E78)?;
        let player_ptr = ps.read::<usize>(self.world_char_man + 0x80)?;
        self.player_ins = PlayerIns::init(player_ptr, ps)?;
        self.session_info_man = SessionInfoMan::init(self.world_char_man, ps)?;
        self.player_ins.refresh_data(ps)?;
        self.session_info_man.refresh_data(self.world_char_man, ps)
    }
}

#[derive(Debug, Clone, Default)]
pub struct PlayerIns {
    player_ins: usize,
    sprj_chr_data_module: usize,

    // Data
    pub chr_stats: ChrStats,
    pub player_game_data: PlayerGameDataMan,
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(packed)]
pub struct ChrStats {
    pub hp: u32,
    pub max_hp: u32,
    pub base_max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub base_max_mp: u32,
    pub sp: u32,
    pub max_sp: u32,
    pub base_max_sp: u32,
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(packed)]
pub struct ChrAttributes {
    pub vigor: u32,
    pub attunement: u32,
    pub endurance: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub faith: u32,
    pub luck: u32,
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub vitality: u32,
    pub soul_level: u32,
    pub unknown_3: u32,
    pub unknown_4: u32,
    pub unknown_5: u32,
    pub unknown_6: u32,
    pub unknown_7: u32,
    pub name_bytes: [u16; 16],
}

impl ChrAttributes {
    pub fn name_string(&self) -> String {
        let name_bytes = self.name_bytes;
        String::from_utf16_lossy(name_bytes.as_ref())
            .trim_matches('\0')
            .to_string()
    }
    pub fn vigor_string(&self) -> String {
        let v = self.vigor;
        v.to_string()
    }
    pub fn attunement_string(&self) -> String {
        let v = self.attunement;
        v.to_string()
    }
    pub fn endurance_string(&self) -> String {
        let v = self.endurance;
        v.to_string()
    }
    pub fn strength_string(&self) -> String {
        let v = self.strength;
        v.to_string()
    }
    pub fn dexterity_string(&self) -> String {
        let v = self.dexterity;
        v.to_string()
    }

    pub fn intelligence_string(&self) -> String {
        let v = self.intelligence;
        v.to_string()
    }
    pub fn faith_string(&self) -> String {
        let v = self.faith;
        v.to_string()
    }
    pub fn luck_string(&self) -> String {
        let v = self.luck;
        v.to_string()
    }
    pub fn vitality_string(&self) -> String {
        let v = self.vitality;
        v.to_string()
    }
    pub fn soul_level_string(&self) -> String {
        let v = self.soul_level;
        v.to_string()
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub struct PlayerGameDataMan {
    player_game_data: usize,

    pub data: PlayerGameData,
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(packed)]
pub struct PlayerGameData {
    pub hp: u32,
    pub max_hp: u32,
    pub base_max_hp: u32,
    pub mp: u32,
    pub max_mp: u32,
    pub base_max_mp: u32,
    pub max_sp: u32,
    pub sp: u32,
    pub base_max_sp: u32,
    pub unknown_1: u32,
    pub unknown_2: u32,
    pub attributes: ChrAttributes,
}

impl PlayerGameDataMan {
    pub fn init(player_game_data: usize, _ps: &Process) -> Result<PlayerGameDataMan> {
        let mut man = PlayerGameDataMan::default();
        man.player_game_data = player_game_data;
        Ok(man)
    }

    pub fn refresh_data(&mut self, ps: &Process) -> Result<()> {
        self.data = ps.read::<PlayerGameData>(self.player_game_data)?;
        Ok(())
    }
}

impl PlayerIns {
    pub fn init(player_ins: usize, ps: &Process) -> Result<PlayerIns> {
        let mut man = PlayerIns::default();
        man.player_ins = player_ins;
        man.sprj_chr_data_module =
            ps.read::<usize>(ps.read::<usize>(player_ins + 0x1F90)? + 0x18)?;
        man.player_game_data =
            PlayerGameDataMan::init(ps.read::<usize>(player_ins + 0x1FA0)? + 0x18, ps)?;
        Ok(man)
    }

    pub fn refresh_data(&mut self, ps: &Process) -> Result<()> {
        self.chr_stats = ps.read::<ChrStats>(self.sprj_chr_data_module + 0xd8)?;
        self.player_game_data.refresh_data(ps)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionInfoMan {
    misc_player_count: usize,
    misc_phantoms_count: usize,
    world_char_man: usize,
    players_base: usize,

    // Data
    pub players: Vec<PlayerIns>,
}

impl SessionInfoMan {
    pub fn init(world_char_man: usize, ps: &Process) -> Result<SessionInfoMan> {
        let mut man = SessionInfoMan::default();
        let base = ps
            .get_module(PROCESS_NAME)
            .ok_or(ProcessError::ModuleNotFound)?
            .base;
        man.misc_player_count = ps.read::<usize>(base + 0x4743AB0)? + 0xD38;
        man.misc_phantoms_count = ps.read::<usize>(base + 0x4743AB0)? + 0xD28;
        man.world_char_man = world_char_man;

        Ok(man)
    }

    pub fn refresh_data(&mut self, world_char_man: usize, ps: &Process) -> Result<()> {
        self.world_char_man = world_char_man;
        self.players_base = ps.read::<usize>(self.world_char_man + 0x40)?;
        let online_players_count = ps.read::<u32>(self.misc_player_count)?;
        let mut players = Vec::new();
        for i in 0..online_players_count {
            let offset = i * 0x38;
            let player_ins_ptr = ps.read::<usize>(self.players_base + offset as usize)?;
            let mut player_ins = PlayerIns::init(player_ins_ptr, ps)?;
            player_ins.refresh_data(ps)?;
            players.push(player_ins);
        }
        self.players = players;
        Ok(())
    }
}
