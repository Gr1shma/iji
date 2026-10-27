use std::path::{Path, PathBuf};

use super::{BuildSystem, staged};
use crate::{Result, util::process::run};

const BUILD_SUBDIR: &str = "meson-build";

pub struct Meson;

impl BuildSystem for Meson {
    fn name(&self) -> &'static str {
        "meson"
    }

    fn build(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<()> {
        staged::reset(stage)?;
        let build = stage.join(BUILD_SUBDIR).display().to_string();
        let source = src.display().to_string();
        let prefix = format!("--prefix={}", staged::PREFIX);

        run(
            "meson",
            &[
                "setup",
                "--buildtype=release",
                prefix.as_str(),
                build.as_str(),
                source.as_str(),
            ],
            src,
        )?;

        let mut compile = vec!["compile", "-C", build.as_str()];
        if let Some(t) = target {
            compile.push(t);
        }
        run("meson", &compile, src)?;

        let destdir = stage.display().to_string();
        run(
            "meson",
            &[
                "install",
                "-C",
                build.as_str(),
                "--destdir",
                destdir.as_str(),
            ],
            src,
        )
    }

    fn artifacts(&self, _src: &Path, stage: &Path, _target: Option<&str>) -> Result<Vec<PathBuf>> {
        staged::executables(stage)
    }
}

pub(super) fn is_meson_project(src: &Path) -> bool {
    src.join("meson.build").is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_only_with_meson_build_file() {
        let dir = std::env::temp_dir().join(format!("iji-meson-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert!(!is_meson_project(&dir));
        std::fs::write(dir.join("meson.build"), "project('x')").unwrap();
        assert!(is_meson_project(&dir));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
