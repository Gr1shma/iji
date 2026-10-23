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

    #[error("invalid config: {0}")]
    InvalidConfig(String),

    #[error("no url for `{0}`; pass a git URL or add a `[repos.{0}]` entry to iji.toml")]
    NoUrl(String),

    #[error("{} is already installed by `{owner}`; use --force to overwrite", .path.display())]
    FileConflict { path: PathBuf, owner: String },

    #[error("{0} package(s) failed")]
    PackagesFailed(usize),

    #[error("invalid build artifact `{}`: no file name", .0.display())]
    InvalidArtifact(PathBuf),

    #[error("expected binary not found after build: {}", .0.display())]
    NoBinaries(PathBuf),

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
    TomlSer(#[from] toml::ser::Error),

    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, IjiError>;
