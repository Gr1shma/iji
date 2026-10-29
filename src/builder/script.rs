use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::util::process;
use crate::{IjiError, Result};

use super::{BuildSystem, staged};

pub enum ScriptSource {
    Inline(String),
    File(PathBuf),
}

pub struct Script {
    source: ScriptSource,
}

impl Script {
    pub fn new(source: ScriptSource) -> Self {
        Self { source }
    }
}

impl BuildSystem for Script {
    fn name(&self) -> &'static str {
        "script"
    }

    fn build(&self, src: &Path, stage: &Path, target: Option<&str>) -> Result<()> {
        if target.is_some() {
            return Err(IjiError::InvalidSpec(
                "a target cannot be combined with a custom script".into(),
            ));
        }
        if let ScriptSource::File(path) = &self.source {
            if !path.is_file() {
                return Err(IjiError::InvalidConfig(format!(
                    "script {} does not exist",
                    path.display()
                )));
            }
        }
        staged::reset(stage)?;

        let args: Vec<&OsStr> = match &self.source {
            ScriptSource::Inline(text) => vec![
                OsStr::new("-euo"),
                OsStr::new("pipefail"),
                OsStr::new("-c"),
                OsStr::new(text),
            ],
            ScriptSource::File(path) => vec![path.as_os_str()],
        };
        let env: [(&str, &OsStr); 3] = [
            ("IJI_SRC", src.as_os_str()),
            ("IJI_STAGE", stage.as_os_str()),
            ("IJI_PREFIX", OsStr::new(staged::PREFIX)),
        ];
        process::run_env("bash", &args, src, &env)
    }

    fn artifacts(&self, _src: &Path, stage: &Path, _target: Option<&str>) -> Result<Vec<PathBuf>> {
        staged::executables(stage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn inline_script_installs_into_stage_and_rejects_target() {
        let root = std::env::temp_dir().join(format!("iji-script-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let src = root.join("src");
        let stage = root.join("stage");
        fs::create_dir_all(&src).unwrap();

        let script = Script::new(ScriptSource::Inline(
            r#"mkdir -p "$IJI_STAGE/usr/bin"
printf '#!/bin/sh\n' > "$IJI_STAGE/usr/bin/tool"
chmod +x "$IJI_STAGE/usr/bin/tool""#
                .into(),
        ));
        script.build(&src, &stage, None).unwrap();
        let found = script.artifacts(&src, &stage, None).unwrap();
        assert_eq!(found, [stage.join("usr/bin/tool")]);

        assert!(matches!(
            script.build(&src, &stage, Some("t")),
            Err(IjiError::InvalidSpec(_))
        ));

        let failing = Script::new(ScriptSource::Inline("exit 1".into()));
        assert!(failing.build(&src, &stage, None).is_err());

        fs::remove_dir_all(&root).unwrap();
    }
}
