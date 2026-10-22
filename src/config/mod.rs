mod path;

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use path::{path_var_contains, resolve};

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
    fn ensure_dirs_is_idempotent() {
        let tmp = env::temp_dir().join(format!("iji-test-{}", std::process::id()));
        let c = Config::with_root(&tmp);
        c.ensure_dirs().unwrap();
        c.ensure_dirs().unwrap();
        assert!(c.bin_dir.is_dir());
        fs::remove_dir_all(&tmp).unwrap();
    }
}
