mod cargo;
mod cmake;
mod go;
mod make;
mod meson;
mod staged;

pub use cargo::Cargo;
pub use cmake::CMake;
pub use go::Go;
pub use make::Make;
pub use meson::Meson;

use std::path::{Path, PathBuf};

use crate::{IjiError, Result};

pub trait BuildSystem {
    fn name(&self) -> &'static str;

    fn build(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<()>;

    fn artifacts(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<Vec<PathBuf>>;
}

pub fn detect(src: &Path) -> Result<Box<dyn BuildSystem>> {
    let marked = |file: &str| src.join(file).is_file();

    if marked("Cargo.toml") {
        Ok(Box::new(Cargo))
    } else if go::is_go_project(src) {
        Ok(Box::new(Go))
    } else if meson::is_meson_project(src) {
        Ok(Box::new(Meson))
    } else if cmake::is_cmake_project(src) {
        Ok(Box::new(CMake))
    } else if make::is_make_project(src) {
        Ok(Box::new(Make))
    } else {
        Err(IjiError::NoBuildSystem(src.display().to_string()))
    }
}
