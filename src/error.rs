use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IjiError {
    #[error("invalid package spec: {0}")]
    InvalidSpec(String),

    #[error("package not installed: {0}")]
    NotInstalled(String),

    #[error("no supported build system found in {0}")]
    NoBuildSystem(String),

    #[error("command failed: {0}")]
    CommandFailed(String),

    #[error("no HOME dir: set the relevant IJI_* env var or $HOME")]
    NoHomeDir,

    #[error("{var} must be an absolute path (got {path:?})")]
    RelativePath { var: String, path: PathBuf },

    #[error("could not make {path:?} absolute: {source}")]
    PathResolve {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not create directory {path:?}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, IjiError>;
