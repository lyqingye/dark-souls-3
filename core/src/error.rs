use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Module Not Found")]
    ModuleNotFound,

    #[error("Failed To Read Memory! Address: {0:#x}")]
    ReadMemoryFail(usize),

    #[error("Failed To Query Memory! Address: {0:#x}")]
    QueryMemoryFail(usize),

    #[error("Process Not Found! Name: {0}")]
    ProcessNotFound(String),

    #[error("Failed To Create File Mapping Name: {0}")]
    CreateFileMapping(String),

    #[error("Failed To Create Mutex")]
    CreateMutex {
        #[from]
        source: std::io::Error,
    },
}

#[derive(Error, Debug)]
pub enum ShMemQError {

    #[error("Failed to create share memory queue!")]
    CreateShMemQ,

    #[error("Failed to file: {0} {1}")]
    CreateFile(PathBuf,u32),

    #[error("File Already exists: {0}")]
    FileExists(PathBuf),

    #[error("Failed To Create File Mapping Name: {0}")]
    CreateFileMapping(PathBuf),

    #[error("Share Memory Message Queue Has Full")]
    HasFull,

    #[error("Invalid Share Memory Message Queue")]
    InvalidMq,
}