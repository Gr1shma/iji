mod cargo;
mod cmake;
mod make;
mod staged;

pub use cargo::Cargo;
pub use cmake::CMake;
pub use make::Make;

use std::path::{Path, PathBuf};

use crate::{IjiError, Result};

pub trait BuildSystem {
    fn name(&self) -> &'static str;

    fn build(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<()>;

    fn artifacts(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<Vec<PathBuf>>;
}

pub fn detect(src: &Path) -> Result<Box<dyn BuildSystem>> {
    let marked = |file: &str| src.join(file).is_file();

    // TODO: go
    if marked("Cargo.toml") {
        Ok(Box::new(Cargo))
    } else if cmake::is_cmake_project(src) {
        Ok(Box::new(CMake))
    } else if make::is_make_project(src) {
        Ok(Box::new(Make))
    } else {
        Err(IjiError::NoBuildSystem(src.display().to_string()))
    }
}
