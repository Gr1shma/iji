use std::fs;
use std::path::{Path, PathBuf};

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

/// Builds the fetched source and places its artifacts, returning the installed files.
pub(crate) fn build_and_place(
    cfg: &Config,
    name: &str,
    target: Option<&str>,
    installed: &[InstalledPackage],
) -> Result<Vec<PathBuf>> {
    let src = cfg.source_dir(name);
    let stage = cfg.build_dir(name);
    let builder = detect(&src)?;
    log::info(format!("building {name} with {}", builder.name()));
    builder.build(&src, &stage, target)?;

    let artifacts = builder.artifacts(&src, &stage, target)?;
    check_conflicts(installed, name, &artifacts, &cfg.bin_dir)?;
    place(&artifacts, &cfg.bin_dir)
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
