use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use super::BuildSystem;
use crate::{IjiError, Result, util::process::run};

const PREFIX: &str = "/usr";
const MAKEFILES: [&str; 3] = ["GNUmakefile", "makefile", "Makefile"];

pub struct Make;

impl BuildSystem for Make {
    fn name(&self) -> &'static str {
        "make"
    }

    fn build(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<()> {
        let goals: Vec<&str> = target.into_iter().collect();
        run("make", &goals, src)?;

        if stage.exists() {
            fs::remove_dir_all(stage)?;
        }
        fs::create_dir_all(stage)?;

        let destdir = format!("DESTDIR={}", stage.display());
        let upper = format!("PREFIX={PREFIX}");
        let lower = format!("prefix={PREFIX}");
        run(
            "make",
            &["install", destdir.as_str(), upper.as_str(), lower.as_str()],
            src,
        )
    }

    fn artifacts(&self, _src: &Path, stage: &Path, _target: Option<&str>) -> Result<Vec<PathBuf>> {
        let dir = stage.join(PREFIX.trim_start_matches('/')).join("bin");
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(IjiError::NoBinaries(dir));
            }
            Err(e) => return Err(e.into()),
        };

        let mut out: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| is_executable(p))
            .collect();
        if out.is_empty() {
            return Err(IjiError::NoBinaries(dir));
        }
        out.sort();
        Ok(out)
    }
}

pub(super) fn is_make_project(src: &Path) -> bool {
    MAKEFILES.iter().any(|f| src.join(f).is_file())
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAKEFILE: &str = "all:\n\ninstall:\n\
        \tmkdir -p $(DESTDIR)$(PREFIX)/bin\n\
        \tprintf '#!/bin/sh\\n' > $(DESTDIR)$(PREFIX)/bin/hi\n\
        \tchmod +x $(DESTDIR)$(PREFIX)/bin/hi\n";

    fn scratch(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("iji-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn installs_into_stage_and_lists_executables() {
        let root = scratch("make-build");
        let (src, stage) = (root.join("src"), root.join("stage"));
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("Makefile"), MAKEFILE).unwrap();

        Make.build(&src, &stage, None).unwrap();
        let got = Make.artifacts(&src, &stage, None).unwrap();
        assert_eq!(got, [stage.join("usr/bin/hi")]);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn artifacts_skip_non_executables() {
        let root = scratch("make-artifacts");
        let bin = root.join("usr/bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("README"), "").unwrap();
        let tool = bin.join("tool");
        fs::write(&tool, "").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(Make.artifacts(&root, &root, None).unwrap(), [tool.clone()]);

        fs::remove_file(&tool).unwrap();
        assert!(matches!(
            Make.artifacts(&root, &root, None),
            Err(IjiError::NoBinaries(_))
        ));

        fs::remove_dir_all(&root).unwrap();
    }
}
