use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use crate::{IjiError, Result};

pub(super) const PREFIX: &str = "/usr";

pub(super) fn reset(stage: &Path) -> Result<()> {
    if stage.exists() {
        fs::remove_dir_all(stage)?;
    }
    fs::create_dir_all(stage)?;
    Ok(())
}

pub(super) fn root(stage: &Path) -> PathBuf {
    stage.join(PREFIX.trim_start_matches('/'))
}

pub(super) fn executables(stage: &Path) -> Result<Vec<PathBuf>> {
    let dir = root(stage).join("bin");
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

fn is_executable(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn executables_skip_non_executables() {
        let stage = std::env::temp_dir().join(format!("iji-staged-{}", std::process::id()));
        let _ = fs::remove_dir_all(&stage);
        let bin = stage.join("usr/bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("README"), "").unwrap();
        let tool = bin.join("tool");
        fs::write(&tool, "").unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(executables(&stage).unwrap(), [tool.clone()]);

        fs::remove_file(&tool).unwrap();
        assert!(matches!(executables(&stage), Err(IjiError::NoBinaries(_))));

        fs::remove_dir_all(&stage).unwrap();
    }
}
