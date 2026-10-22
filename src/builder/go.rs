use std::{
    fs,
    path::{Path, PathBuf},
};

use super::{BuildSystem, staged};
use crate::{Result, util::process::run};

pub struct Go;

impl BuildSystem for Go {
    fn name(&self) -> &'static str {
        "go"
    }

    fn build(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<()> {
        staged::reset(stage)?;
        let bin = staged::root(stage).join("bin");
        fs::create_dir_all(&bin)?;

        let out = bin.display().to_string();
        let pkg = target.map_or_else(|| "./...".to_owned(), package_pattern);
        run("go", &["build", "-o", out.as_str(), pkg.as_str()], src)
    }

    fn artifacts(&self, _src: &Path, stage: &Path, _target: Option<&str>) -> Result<Vec<PathBuf>> {
        staged::executables(stage)
    }
}

pub(super) fn is_go_project(src: &Path) -> bool {
    src.join("go.mod").is_file()
}

fn package_pattern(target: &str) -> String {
    format!("./{}", target.trim_start_matches("./"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_is_always_a_relative_package_path() {
        assert_eq!(package_pattern("cmd/foo"), "./cmd/foo");
        assert_eq!(package_pattern("./cmd/foo"), "./cmd/foo");
        assert_eq!(package_pattern("-toolexec=evil"), "./-toolexec=evil");
    }

    #[test]
    fn builds_main_packages_into_stage() {
        let root = std::env::temp_dir().join(format!("iji-go-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (src, stage) = (root.join("src"), root.join("stage"));
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("go.mod"), "module example.com/hi\n\ngo 1.21\n").unwrap();
        fs::write(src.join("main.go"), "package main\n\nfunc main() {}\n").unwrap();

        Go.build(&src, &stage, None).unwrap();
        let got = Go.artifacts(&src, &stage, None).unwrap();
        assert_eq!(got, [stage.join("usr/bin/hi")]);

        fs::remove_dir_all(&root).unwrap();
    }
}
