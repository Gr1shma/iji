use std::{
    env,
    ffi::OsString,
    fs,
    path::{self, Path, PathBuf},
};

use crate::{IjiError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub cache_dir: PathBuf,
}

const CONFIG_FILE: &str = "iji.toml";
const DB_FILE: &str = "installed.toml";

fn non_empty_path(value: Option<OsString>) -> Option<PathBuf> {
    value.filter(|s| !s.is_empty()).map(PathBuf::from)
}

fn expand_tilde(p: PathBuf) -> PathBuf {
    let Some(home) = dirs::home_dir() else {
        return p;
    };
    match p.strip_prefix("~") {
        Ok(rest) => home.join(rest),
        Err(_) => p,
    }
}

fn resolve_with(
    env_key: &str,
    env_value: Option<OsString>,
    fallback: Option<PathBuf>,
) -> Result<PathBuf> {
    match non_empty_path(env_value) {
        Some(p) => {
            let p = expand_tilde(p);
            if p.is_relative() {
                return Err(IjiError::RelativePath {
                    var: env_key.to_owned(),
                    path: p,
                });
            }
            path::absolute(&p).map_err(|source| IjiError::PathResolve { path: p, source })
        }
        None => fallback.ok_or(IjiError::NoHomeDir),
    }
}

fn resolve(env_key: &str, fallback: Option<PathBuf>) -> Result<PathBuf> {
    resolve_with(env_key, env::var_os(env_key), fallback)
}

fn path_var_contains(path_var: Option<OsString>, dir: &Path) -> bool {
    path_var.is_some_and(|v| env::split_paths(&v).any(|p| p == dir))
}

impl Config {
    pub fn load() -> Result<Self> {
        Ok(Self {
            config_dir: resolve("IJI_CONFIG_DIR", dirs::config_dir().map(|p| p.join("iji")))?,
            data_dir: resolve("IJI_DATA_DIR", dirs::data_dir().map(|p| p.join("iji")))?,
            bin_dir: resolve(
                "IJI_BIN_DIR",
                dirs::home_dir().map(|p| p.join(".local").join("bin")),
            )?,
            cache_dir: resolve("IJI_CACHE_DIR", dirs::cache_dir().map(|p| p.join("iji")))?,
        })
    }

    pub fn with_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            bin_dir: root.join("bin"),
            cache_dir: root.join("cache"),
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            &self.config_dir,
            &self.data_dir,
            &self.bin_dir,
            &self.cache_dir,
        ] {
            fs::create_dir_all(dir).map_err(|source| IjiError::CreateDir {
                path: dir.clone(),
                source,
            })?;
        }
        Ok(())
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE)
    }

    pub fn db_file(&self) -> PathBuf {
        self.data_dir.join(DB_FILE)
    }

    pub fn source_dir(&self, package: &str) -> PathBuf {
        self.cache_dir.join("src").join(package)
    }

    pub fn build_dir(&self, package: &str) -> PathBuf {
        self.cache_dir.join("build").join(package)
    }

    pub fn bin_dir_on_path(&self) -> bool {
        path_var_contains(env::var_os("PATH"), &self.bin_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(s: &str) -> Option<OsString> {
        Some(OsString::from(s))
    }

    #[test]
    fn unset_env_uses_fallback() {
        let got = resolve_with("IJI_X", None, Some("/fallback".into())).unwrap();
        assert_eq!(got, PathBuf::from("/fallback"));
    }

    #[test]
    fn empty_env_uses_fallback() {
        let got = resolve_with("IJI_X", os(""), Some("/fallback".into())).unwrap();
        assert_eq!(got, PathBuf::from("/fallback"));
    }

    #[test]
    fn no_env_and_no_fallback_errors() {
        let err = resolve_with("IJI_X", None, None).unwrap_err();
        assert!(matches!(err, IjiError::NoHomeDir));
    }

    #[cfg(unix)]
    #[test]
    fn absolute_env_wins_over_fallback() {
        let got = resolve_with("IJI_X", os("/tmp/x"), Some("/fallback".into())).unwrap();
        assert_eq!(got, PathBuf::from("/tmp/x"));
    }

    #[test]
    fn relative_env_is_rejected() {
        let err = resolve_with("IJI_X", os("bin"), Some("/fallback".into())).unwrap_err();
        assert!(matches!(err, IjiError::RelativePath { .. }));
    }

    #[test]
    fn tilde_is_expanded_to_home() {
        if let Some(home) = dirs::home_dir() {
            assert_eq!(expand_tilde(PathBuf::from("~")), home);
            assert_eq!(expand_tilde(PathBuf::from("~/bin")), home.join("bin"));
        }
    }

    #[test]
    fn tilde_prefix_without_slash_is_untouched() {
        assert_eq!(expand_tilde(PathBuf::from("~foo")), PathBuf::from("~foo"));
    }

    #[test]
    fn with_root_nests_everything() {
        let c = Config::with_root("/r");
        assert_eq!(c.bin_dir, PathBuf::from("/r/bin"));
        assert_eq!(c.cache_dir, PathBuf::from("/r/cache"));
    }

    #[test]
    fn derived_paths() {
        let c = Config::with_root("/r");
        assert_eq!(c.config_file(), PathBuf::from("/r/config/iji.toml"));
        assert_eq!(c.db_file(), PathBuf::from("/r/data/installed.toml"));
        assert_eq!(c.source_dir("foo"), PathBuf::from("/r/cache/src/foo"));
        assert_eq!(c.build_dir("foo"), PathBuf::from("/r/cache/build/foo"));
    }

    #[test]
    fn path_var_detection() {
        let v = env::join_paths(["/usr/bin", "/r/bin"]).unwrap();
        assert!(path_var_contains(Some(v.clone()), Path::new("/r/bin")));
        assert!(!path_var_contains(Some(v), Path::new("/other")));
        assert!(!path_var_contains(None, Path::new("/r/bin")));
    }

    #[test]
    fn ensure_dirs_is_idempotent() {
        let tmp = env::temp_dir().join(format!("iji-test-{}", std::process::id()));
        let c = Config::with_root(&tmp);
        c.ensure_dirs().unwrap();
        c.ensure_dirs().unwrap();
        assert!(c.bin_dir.is_dir());
        fs::remove_dir_all(&tmp).unwrap();
    }
}
