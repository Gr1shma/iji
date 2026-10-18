mod cargo;

pub use cargo::Cargo;

use std::path::{Path, PathBuf};

use crate::{IjiError, Result};

pub trait BuildSystem {
    fn name(&self) -> &'static str;

    fn build(&self, src: &Path, target: Option<&str>) -> Result<()>;

    fn artifacts(&self, src: &Path, target: Option<&str>) -> Result<Vec<PathBuf>>;
}

pub fn detect(src: &Path) -> Result<Box<dyn BuildSystem>> {
    let marked = |file: &str| src.join(file).is_file();

    // TODO: cmake, make, go
    if marked("Cargo.toml") {
        Ok(Box::new(Cargo))
    } else {
        Err(IjiError::NoBuildSystem(src.display().to_string()))
    }
}
