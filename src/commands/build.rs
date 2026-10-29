use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::builder::BuildSystem;
use crate::builder::Script;
use crate::builder::ScriptSource;
use crate::builder::detect;
use crate::config::Config;
use crate::installer::place;
use crate::package::InstalledPackage;
use crate::util::log;
use crate::vcs::{Git, Vcs};
use crate::{IjiError, Result};

/// Clones `url` into the package's source dir and returns the checked-out commit.
pub(crate) fn fetch(cfg: &Config, name: &str, url: &str, version: Option<&str>) -> Result<String> {
    let src = cfg.source_dir(name);
    if src.exists() {
        fs::remove_dir_all(&src)?;
    }
    Git.fetch(url, &src, version)?;
    Git.head(&src)
}

/// Picks the configured script for custom packages, else detects the build system.
fn select(cfg: &Config, name: &str, src: &Path, custom: bool) -> Result<Box<dyn BuildSystem>> {
    if !custom {
        return detect(src);
    }
    let repo = cfg.repos.get(name).ok_or_else(|| {
        IjiError::InvalidConfig(format!(
            "{name} uses a custom script, but repos.{name} is missing from iji.toml"
        ))
    })?;
    let source = match (&repo.run, &repo.script) {
        (Some(text), None) => ScriptSource::Inline(text.clone()),
        (None, Some(path)) => ScriptSource::File(cfg.config_dir.join(path)),
        _ => {
            return Err(IjiError::InvalidConfig(format!(
                "repos.{name} has no `run` or `script`"
            )));
        }
    };
    Ok(Box::new(Script::new(source)))
}

/// Builds the fetched source and places its artifacts, returning the installed files.
pub(crate) fn build_and_place(
    cfg: &Config,
    name: &str,
    target: Option<&str>,
    custom: bool,
    installed: &[InstalledPackage],
) -> Result<Vec<PathBuf>> {
    let src = cfg.source_dir(name);
    let stage = cfg.build_dir(name);
    let builder = select(cfg, name, &src, custom)?;
    log::info(format!("building {name} with {}", builder.name()));
    builder.build(&src, &stage, target)?;

    let artifacts = builder.artifacts(&src, &stage, target)?;
    check_conflicts(installed, name, &artifacts, &cfg.bin_dir)?;
    place(&artifacts, &cfg.bin_dir)
}

/// True if `path` is absolute and has no `..` component.
pub(crate) fn is_safe_recorded_path(path: &Path) -> bool {
    path.is_absolute() && !path.components().any(|c| matches!(c, Component::ParentDir))
}

/// Removes files from the old install that the new install no longer provides.
pub(crate) fn remove_stale(old: &[PathBuf], new_files: &[PathBuf]) {
    for path in old.iter().filter(|f| !new_files.contains(f)) {
        if !is_safe_recorded_path(path) {
            log::warn(format!("refusing to remove {}", path.display()));
            continue;
        }
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => log::warn(format!("could not remove {}: {e}", path.display())),
        }
    }
}

/// Fails if an artifact would overwrite a file owned by a different package.
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
            commit: None,
            custom: false,
            files: files.iter().map(PathBuf::from).collect(),
        }
    }

    #[test]
    fn recorded_paths_must_be_absolute_without_parent_dirs() {
        assert!(is_safe_recorded_path(Path::new("/home/u/.local/bin/x")));
        for bad in ["bin/x", "../../etc/x", "/home/u/../../etc/x"] {
            assert!(!is_safe_recorded_path(Path::new(bad)), "{bad}");
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
