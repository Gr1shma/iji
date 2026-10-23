mod file;
mod path;

use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

use file::ConfigFile;
use path::{path_var_contains, resolve, resolve_file};

use crate::{IjiError, Result, package::PackageSpec};

pub use file::Repo;

#[derive(Debug, Clone)]
pub struct Config {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub source_root: PathBuf,
    pub repos: BTreeMap<String, Repo>,
}

const CONFIG_FILE: &str = "iji.toml";
const DB_FILE: &str = "installed.toml";

impl Config {
    pub fn load() -> Result<Self> {
        let config_dir = resolve("IJI_CONFIG_DIR", dirs::config_dir().map(|p| p.join("iji")))?;
        let file = ConfigFile::read(&config_dir.join(CONFIG_FILE))?;
        let cache_dir = resolve_file(
            "IJI_CACHE_DIR",
            "paths.cache_dir",
            file.paths.cache_dir,
            dirs::cache_dir().map(|p| p.join("iji")),
        )?;

        Ok(Self {
            data_dir: resolve("IJI_DATA_DIR", dirs::data_dir().map(|p| p.join("iji")))?,
            bin_dir: resolve_file(
                "IJI_BIN_DIR",
                "paths.bin_dir",
                file.paths.bin_dir,
                dirs::home_dir().map(|p| p.join(".local").join("bin")),
            )?,
            source_root: resolve_file(
                "IJI_SOURCE_DIR",
                "paths.source_dir",
                file.paths.source_dir,
                Some(cache_dir.join("src")),
            )?,
            cache_dir,
            config_dir,
            repos: file.repos,
        })
    }

    pub fn with_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let cache_dir = root.join("cache");
        Self {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            bin_dir: root.join("bin"),
            source_root: cache_dir.join("src"),
            cache_dir,
            repos: BTreeMap::new(),
        }
    }

    pub fn ensure_dirs(&self) -> Result<()> {
        for dir in [
            &self.config_dir,
            &self.data_dir,
            &self.bin_dir,
            &self.cache_dir,
            &self.source_root,
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
        self.source_root.join(package)
    }

    pub fn build_dir(&self, package: &str) -> PathBuf {
        self.cache_dir.join("build").join(package)
    }

    pub fn bin_dir_on_path(&self) -> bool {
        path_var_contains(env::var_os("PATH"), &self.bin_dir)
    }

    pub fn resolve_spec(&self, mut spec: PackageSpec) -> Result<PackageSpec> {
        if spec.url.is_some() {
            return Ok(spec);
        }

        let Some(repo) = self.repos.get(&spec.name) else {
            return Err(IjiError::NoUrl(spec.name));
        };

        spec.url = Some(repo.url.clone());
        spec.version = spec.version.or_else(|| repo.version.clone());
        spec.target = spec.target.or_else(|| repo.target.clone());
        Ok(spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(text: &str) -> PackageSpec {
        text.parse().unwrap()
    }

    fn config_with_repo(name: &str, repo: Repo) -> Config {
        let mut cfg = Config::with_root("/r");
        cfg.repos.insert(name.to_owned(), repo);
        cfg
    }

    fn repo() -> Repo {
        Repo {
            url: "https://example.com/foo".into(),
            version: Some("v1".into()),
            target: Some("foo-bin".into()),
        }
    }

    #[test]
    fn with_root_nests_everything() {
        let c = Config::with_root("/r");
        assert_eq!(c.bin_dir, PathBuf::from("/r/bin"));
        assert_eq!(c.cache_dir, PathBuf::from("/r/cache"));
        assert_eq!(c.source_root, PathBuf::from("/r/cache/src"));
        assert!(c.repos.is_empty());
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
    fn custom_source_root_changes_clone_location() {
        let mut c = Config::with_root("/r");
        c.source_root = PathBuf::from("/code");
        assert_eq!(c.source_dir("foo"), PathBuf::from("/code/foo"));
    }

    #[test]
    fn ensure_dirs_is_idempotent() {
        let tmp = env::temp_dir().join(format!("iji-test-{}", std::process::id()));
        let c = Config::with_root(&tmp);
        c.ensure_dirs().unwrap();
        c.ensure_dirs().unwrap();
        assert!(c.bin_dir.is_dir());
        assert!(c.source_root.is_dir());
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn bare_name_takes_url_version_and_target_from_repo() {
        let cfg = config_with_repo("foo", repo());
        let got = cfg.resolve_spec(spec("foo")).unwrap();

        assert_eq!(got.url.as_deref(), Some("https://example.com/foo"));
        assert_eq!(got.version.as_deref(), Some("v1"));
        assert_eq!(got.target.as_deref(), Some("foo-bin"));
    }

    #[test]
    fn spec_version_and_target_win_over_repo() {
        let cfg = config_with_repo("foo", repo());
        let got = cfg.resolve_spec(spec("foo@v2,other")).unwrap();

        assert_eq!(got.url.as_deref(), Some("https://example.com/foo"));
        assert_eq!(got.version.as_deref(), Some("v2"));
        assert_eq!(got.target.as_deref(), Some("other"));
    }

    #[test]
    fn url_spec_does_not_look_up_repos() {
        let cfg = config_with_repo("foo", repo());
        let got = cfg.resolve_spec(spec("https://example.com/bar")).unwrap();

        assert_eq!(got.name, "bar");
        assert_eq!(got.url.as_deref(), Some("https://example.com/bar"));
        assert_eq!(got.version, None);
        assert_eq!(got.target, None);
    }

    #[test]
    fn unknown_bare_name_errors() {
        let cfg = Config::with_root("/r");
        let err = cfg.resolve_spec(spec("foo")).unwrap_err();
        assert!(matches!(err, IjiError::NoUrl(name) if name == "foo"));
    }

    #[test]
    fn repo_version_pins_the_record() {
        let cfg = config_with_repo("foo", repo());
        let got = cfg.resolve_spec(spec("foo")).unwrap();
        assert_eq!(got.version.as_deref(), Some("v1"));
    }
}
