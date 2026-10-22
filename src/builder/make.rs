use std::path::{Path, PathBuf};

use super::{BuildSystem, staged};
use crate::{Result, util::process::run};

const MAKEFILES: [&str; 3] = ["GNUmakefile", "makefile", "Makefile"];

pub struct Make;

impl BuildSystem for Make {
    fn name(&self) -> &'static str {
        "make"
    }

    fn build(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<()> {
        let goals: Vec<&str> = target.into_iter().collect();
        run("make", &goals, src)?;

        staged::reset(stage)?;

        let destdir = format!("DESTDIR={}", stage.display());
        let upper = format!("PREFIX={}", staged::PREFIX);
        let lower = format!("prefix={}", staged::PREFIX);
        run(
            "make",
            &["install", destdir.as_str(), upper.as_str(), lower.as_str()],
            src,
        )
    }

    fn artifacts(&self, _src: &Path, stage: &Path, _target: Option<&str>) -> Result<Vec<PathBuf>> {
        staged::executables(stage)
    }
}

pub(super) fn is_make_project(src: &Path) -> bool {
    MAKEFILES.iter().any(|f| src.join(f).is_file())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    const MAKEFILE: &str = "all:\n\ninstall:\n\
        \tmkdir -p $(DESTDIR)$(PREFIX)/bin\n\
        \tprintf '#!/bin/sh\\n' > $(DESTDIR)$(PREFIX)/bin/hi\n\
        \tchmod +x $(DESTDIR)$(PREFIX)/bin/hi\n";

    #[test]
    fn installs_into_stage_and_lists_executables() {
        let root = std::env::temp_dir().join(format!("iji-make-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (src, stage) = (root.join("src"), root.join("stage"));
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("Makefile"), MAKEFILE).unwrap();

        Make.build(&src, &stage, None).unwrap();
        let got = Make.artifacts(&src, &stage, None).unwrap();
        assert_eq!(got, [stage.join("usr/bin/hi")]);

        fs::remove_dir_all(&root).unwrap();
    }
}
