use std::path::{Path, PathBuf};

use super::{BuildSystem, staged};
use crate::{Result, util::process::run};

const BUILD_SUBDIR: &str = "cmake-build";

pub struct CMake;

impl BuildSystem for CMake {
    fn name(&self) -> &'static str {
        "cmake"
    }

    fn build(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<()> {
        staged::reset(stage)?;
        let build = stage.join(BUILD_SUBDIR).display().to_string();
        let prefix = staged::root(stage).display().to_string();

        run(
            "cmake",
            &[
                "-S",
                ".",
                "-B",
                build.as_str(),
                "-DCMAKE_BUILD_TYPE=Release",
            ],
            src,
        )?;

        let mut args = vec!["--build", build.as_str()];
        if let Some(t) = target {
            args.extend(["--target", t]);
        }
        run("cmake", &args, src)?;

        run(
            "cmake",
            &["--install", build.as_str(), "--prefix", prefix.as_str()],
            src,
        )
    }

    fn artifacts(&self, _src: &Path, stage: &Path, _target: Option<&str>) -> Result<Vec<PathBuf>> {
        staged::executables(stage)
    }
}

pub(super) fn is_cmake_project(src: &Path) -> bool {
    src.join("CMakeLists.txt").is_file()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn builds_and_installs_into_stage() {
        let root = std::env::temp_dir().join(format!("iji-cmake-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (src, stage) = (root.join("src"), root.join("stage"));
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.15)\nproject(hi NONE)\ninstall(PROGRAMS hi DESTINATION bin)\n",
        )
        .unwrap();
        fs::write(src.join("hi"), "#!/bin/sh\n").unwrap();

        CMake.build(&src, &stage, None).unwrap();
        let got = CMake.artifacts(&src, &stage, None).unwrap();
        assert_eq!(got, [stage.join("usr/bin/hi")]);

        fs::remove_dir_all(&root).unwrap();
    }
}
