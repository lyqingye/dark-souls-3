use anyhow::Result;
use crate::error::ProcessError;
use crate::process::{from_name, Process};

const PROCESS_NAME: &'static str = "DarkSoulsIII.exe";

#[derive(Debug,Clone)]
pub struct GameData {
    ps: Process,
    world_chr_man: WorldChrMan,
}

impl GameData {
    pub fn init() -> Result<GameData> {
        let process = from_name(PROCESS_NAME)
            .ok_or(ProcessError::ProcessNotFound(PROCESS_NAME.to_string()))?;
        let world_chr_man = WorldChrMan::init(&process)?;

        Ok(Self{
            ps: process,
            world_chr_man,
        })
    }

    pub fn refresh_world_char_man_data(&mut self) -> Result<()> {
        self.world_chr_man.refresh_data(&self.ps)
    }
}

#[derive(Debug,Clone,Default)]
pub struct WorldChrMan {
    world_char_man: usize,
    player_ins: usize,
    sprj_chr_data_module: usize,

    // Data
    pub chr_stats: ChrStats,
}


#[derive(Debug,Copy,Clone,Default)]
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

impl WorldChrMan {
    pub fn init(ps: &Process) -> Result<WorldChrMan> {
        let mut man = WorldChrMan::default();
        let base = ps.get_module(PROCESS_NAME)
            .ok_or(ProcessError::ModuleNotFound)?.base;
        man.world_char_man = ps.read::<usize>(base + 0x4768E78)?;
        man.player_ins = ps.read::<usize>(man.world_char_man + 0x80)?;
        let temp = ps.read::<usize>(man.player_ins + 0x1F90)?;
        man.sprj_chr_data_module = ps.read::<usize>(temp + 0x18)?;
        Ok(man)
    }

    pub fn refresh_data(&mut self, ps: &Process) -> Result<()> {
        self.chr_stats = ps.read::<ChrStats>(self.sprj_chr_data_module + 0xd8)?;
        Ok(())
    }
}
