use std::fs;
use std::io;

use super::build;
use crate::config::Config;
use crate::db::{Database, TomlStore};
use crate::package::{InstalledPackage, validate_name};
use crate::util::log;
use crate::{IjiError, Result};

pub fn run(cfg: &Config, names: &[String]) -> Result<()> {
    let mut db = TomlStore::new(cfg.db_file());

    let names: Vec<String> = if names.is_empty() {
        db.all()?.into_iter().map(|p| p.name).collect()
    } else {
        names.to_vec()
    };
    if names.is_empty() {
        log::info("no packages installed");
        return Ok(());
    }

    cfg.ensure_dirs()?;
    let mut failed = 0;
    for name in &names {
        if let Err(e) = update_one(cfg, &mut db, name) {
            log::error(format!("{name}: {e}"));
            failed += 1;
        }
    }

    if failed > 0 {
        return Err(IjiError::PackagesFailed(failed));
    }
    Ok(())
}

fn update_one(cfg: &Config, db: &mut impl Database, name: &str) -> Result<()> {
    validate_name(name)?;
    let old = db
        .get(name)?
        .ok_or_else(|| IjiError::NotInstalled(name.to_owned()))?;

    if let Some(v) = &old.version {
        log::info(format!("{name} is pinned to {v}, skipping"));
        return Ok(());
    }

    log::info(format!("checking {name} for updates"));
    let commit = build::fetch(cfg, name, &old.url, None)?;
    if old.commit.as_deref() == Some(commit.as_str()) {
        log::info(format!("{name} is up to date"));
        return Ok(());
    }

    let installed = db.all()?;
    let files = build::build_and_place(cfg, name, old.target.as_deref(), &installed)?;
    remove_stale(&old, &files);

    db.insert(InstalledPackage {
        commit: Some(commit),
        files,
        ..old
    })?;
    log::success(format!("updated {name}"));
    Ok(())
}

fn remove_stale(old: &InstalledPackage, new_files: &[std::path::PathBuf]) {
    for path in old.files.iter().filter(|f| !new_files.contains(f)) {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => log::warn(format!("could not remove {}: {e}", path.display())),
        }
    }
}
