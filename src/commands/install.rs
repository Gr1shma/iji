use std::fs;
use std::path::{Path, PathBuf};

use crate::builder::detect;
use crate::config::Config;
use crate::db::{Database, TomlStore};
use crate::installer::place;
use crate::package::{InstalledPackage, PackageSpec};
use crate::util::log;
use crate::vcs::{Git, Vcs};
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
    let spec: PackageSpec = arg.parse()?;
    let name = spec.name.as_str();

    if !force && db.get(name)?.is_some() {
        log::info(format!(
            "{name} is already installed (use --force to reinstall)"
        ));
        return Ok(false);
    }

    let url = spec
        .url
        .clone()
        .ok_or_else(|| IjiError::NoUrl(name.to_owned()))?;
    let target = spec.target.as_deref();

    let src = cfg.source_dir(name);
    if src.exists() {
        fs::remove_dir_all(&src)?;
    }

    log::info(format!("fetching {name} from {url}"));
    Git.fetch(&url, &src, spec.version.as_deref())?;

    let builder = detect(&src)?;
    log::info(format!("building {name} with {}", builder.name()));
    builder.build(&src, target)?;

    let artifacts = builder.artifacts(&src, target)?;
    if !force {
        check_conflicts(&db.all()?, name, &artifacts, &cfg.bin_dir)?;
    }
    let files = place(&artifacts, &cfg.bin_dir)?;

    db.insert(InstalledPackage {
        name: name.to_owned(),
        url,
        version: spec.version,
        target: spec.target,
        files,
    })?;
    log::success(format!("installed {name}"));
    Ok(true)
}

fn check_conflicts(
    installed: &[InstalledPackage],
    name: &str,
    artifacts: &[PathBuf],
    bin_dir: &Path,
) -> Result<()> {
    for file in artifacts.iter().filter_map(|a| a.file_name()) {
        let dest = bin_dir.join(file);
        if let Some(owner) = installed
            .iter()
            .find(|p| p.name != name && p.files.contains(&dest))
        {
            return Err(IjiError::FileConflict {
                path: dest,
                owner: owner.name.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, files: &[&str]) -> InstalledPackage {
        InstalledPackage {
            name: name.into(),
            url: "u".into(),
            version: None,
            target: None,
            files: files.iter().map(PathBuf::from).collect(),
        }
    }

    #[test]
    fn refuses_to_overwrite_another_packages_file() {
        let installed = [pkg("a", &["/bin/tool"])];
        let artifacts = [PathBuf::from("/src/target/release/tool")];
        let err = check_conflicts(&installed, "b", &artifacts, Path::new("/bin")).unwrap_err();
        assert!(matches!(err, IjiError::FileConflict { .. }));
    }

    #[test]
    fn allows_overwriting_its_own_files() {
        let installed = [pkg("a", &["/bin/tool"])];
        let artifacts = [PathBuf::from("/src/target/release/tool")];
        assert!(check_conflicts(&installed, "a", &artifacts, Path::new("/bin")).is_ok());
    }
}
