use std::fs;
use std::io;
use std::path::PathBuf;

use crate::config::Config;
use crate::db::{Database, TomlStore};
use crate::package::validate_name;
use crate::util::log;
use crate::{IjiError, Result};

pub fn run(cfg: &Config, names: &[String]) -> Result<()> {
    let mut db = TomlStore::new(cfg.db_file());

    let mut failed = 0;
    for name in names {
        if let Err(e) = remove_one(cfg, &mut db, name) {
            log::error(format!("{name}: {e}"));
            failed += 1;
        }
    }

    if failed > 0 {
        return Err(IjiError::PackagesFailed(failed));
    }
    Ok(())
}

fn remove_one(cfg: &Config, db: &mut impl Database, name: &str) -> Result<()> {
    validate_name(name)?;

    let pkg = db
        .get(name)?
        .ok_or_else(|| IjiError::NotInstalled(name.to_owned()))?;

    remove_files(&pkg.files)?;
    db.remove(name)?;

    let src = cfg.source_dir(name);
    if let Err(e) = remove_dir_if_exists(&src) {
        log::warn(format!("could not remove {}: {e}", src.display()));
    }

    log::success(format!("removed {name}"));
    Ok(())
}

fn remove_files(files: &[PathBuf]) -> Result<()> {
    let mut first_err = None;
    for path in files {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                log::error(format!("could not remove {}: {e}", path.display()));
                first_err.get_or_insert(e);
            }
        }
    }
    first_err.map_or(Ok(()), |e| Err(e.into()))
}

fn remove_dir_if_exists(path: &std::path::Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_existing_files_and_ignores_missing_ones() {
        let dir = std::env::temp_dir().join(format!("iji-remove-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let present = dir.join("present");
        fs::write(&present, "x").unwrap();
        let missing = dir.join("missing");

        remove_files(&[present.clone(), missing]).unwrap();

        assert!(!present.exists());
        fs::remove_dir_all(&dir).unwrap();
    }
}
