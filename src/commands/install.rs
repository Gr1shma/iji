use std::fs;
use std::io;

use crate::commands::build;
use crate::config::Config;
use crate::db::{Database, TomlStore};
use crate::package::{InstalledPackage, PackageSpec};
use crate::util::log;
use crate::{IjiError, Result};

pub fn run(cfg: &Config, packages: &[String], force: bool) -> Result<()> {
    cfg.ensure_dirs()?;
    let mut db = TomlStore::new(cfg.db_file());

    let (mut installed, mut failed) = (0, 0);
    for arg in packages {
        match install_one(cfg, &mut db, arg, force) {
            Ok(true) => installed += 1,
            Ok(false) => {}
            Err(e) => {
                log::error(format!("{arg}: {e}"));
                failed += 1;
            }
        }
    }

    if installed > 0 && !cfg.bin_dir_on_path() {
        log::warn(format!("{} is not on PATH", cfg.bin_dir.display()));
    }
    if failed > 0 {
        return Err(IjiError::PackagesFailed(failed));
    }
    Ok(())
}

fn install_one(cfg: &Config, db: &mut impl Database, arg: &str, force: bool) -> Result<bool> {
    let parsed: PackageSpec = arg.parse()?;
    // Only a bare name can pick up a script from `[repos]`; a URL never does.
    let custom = parsed.url.is_none()
        && cfg
            .repos
            .get(&parsed.name)
            .is_some_and(|r| r.run.is_some() || r.script.is_some());
    let spec = cfg.resolve_spec(parsed)?;
    if custom && spec.target.is_some() {
        return Err(IjiError::InvalidSpec(
            "a target cannot be combined with a custom script".into(),
        ));
    }
    let name = spec.name.as_str();
    let existing = db.get(name)?;

    if !force && existing.is_some() {
        log::info(format!(
            "{name} is already installed (use --force to reinstall)"
        ));
        return Ok(false);
    }

    let first_install = existing.is_none();

    let url = spec
        .url
        .clone()
        .ok_or_else(|| IjiError::NoUrl(name.to_owned()))?;

    log::info(format!("fetching {name} from {url}"));
    let result = (|| {
        let commit = build::fetch(cfg, name, &url, spec.version.as_deref())?;

        // --force skips the conflict check by passing no other packages.
        let others = if force { Vec::new() } else { db.all()? };
        let files = build::build_and_place(cfg, name, spec.target.as_deref(), custom, &others)?;

        if let Some(old) = &existing {
            build::remove_stale(&old.files, &files);
        }

        db.insert(InstalledPackage {
            name: name.to_owned(),
            url,
            version: spec.version,
            target: spec.target,
            commit: Some(commit),
            files,
            custom,
        })
    })();

    if result.is_err() && first_install {
        let src = cfg.source_dir(name);
        match fs::remove_dir_all(&src) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => log::warn(format!("could not remove {}: {e}", src.display())),
        }
    }
    result?;

    log::success(format!("installed {name}"));
    Ok(true)
}
