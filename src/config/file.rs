use std::{
    collections::BTreeMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{IjiError, Result, package::validate_name};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Repo {
    pub url: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Paths {
    #[serde(default)]
    pub bin_dir: Option<PathBuf>,
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,
    #[serde(default)]
    pub source_dir: Option<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConfigFile {
    #[serde(default)]
    pub repos: BTreeMap<String, Repo>,
    #[serde(default)]
    pub paths: Paths,
}

impl ConfigFile {
    pub fn read(path: &Path) -> Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn parse(text: &str) -> Result<Self> {
        let file: Self = toml::from_str(text)?;
        file.validate()?;
        Ok(file)
    }

    fn validate(&self) -> Result<()> {
        for (name, repo) in &self.repos {
            validate_name(name)?;
            if repo.url.trim().is_empty() {
                return Err(IjiError::InvalidConfig(format!(
                    "repos.{name} is missing a `url`"
                )));
            }
            if repo.version.as_deref().is_some_and(|v| v.trim().is_empty()) {
                return Err(IjiError::InvalidConfig(format!(
                    "repos.{name}.version must not be empty"
                )));
            }
            if repo.target.as_deref().is_some_and(|t| t.trim().is_empty()) {
                return Err(IjiError::InvalidConfig(format!(
                    "repos.{name}.target must not be empty"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repos_and_paths() {
        let file = ConfigFile::parse(
            r#"
            [repos.pkgit]
            url = "https://git.symlinx.net/pkgit"

            [repos.mytool]
            url = "https://github.com/u/mytool"
            version = "v1.2"
            target = "mytool"

            [paths]
            bin_dir = "~/.local/bin"
            cache_dir = "~/.cache/iji"
            source_dir = "~/code"
            "#,
        )
        .unwrap();

        let mytool = file.repos.get("mytool").unwrap();
        assert_eq!(mytool.url, "https://github.com/u/mytool");
        assert_eq!(mytool.version.as_deref(), Some("v1.2"));
        assert_eq!(mytool.target.as_deref(), Some("mytool"));

        let pkgit = file.repos.get("pkgit").unwrap();
        assert_eq!(pkgit.version, None);
        assert_eq!(pkgit.target, None);

        assert_eq!(
            file.paths.bin_dir.as_deref(),
            Some(Path::new("~/.local/bin"))
        );
        assert_eq!(
            file.paths.cache_dir.as_deref(),
            Some(Path::new("~/.cache/iji"))
        );
        assert_eq!(file.paths.source_dir.as_deref(), Some(Path::new("~/code")));
    }

    #[test]
    fn empty_file_uses_defaults() {
        let file = ConfigFile::parse("").unwrap();
        assert!(file.repos.is_empty());
        assert_eq!(file.paths.bin_dir, None);
        assert_eq!(file.paths.cache_dir, None);
        assert_eq!(file.paths.source_dir, None);
    }

    #[test]
    fn rejects_invalid_repo_names() {
        for name in ["", ".hidden", "a/b", "../foo", "a b"] {
            let text = format!("[repos.\"{name}\"]\nurl = \"https://example.com/r\"\n");
            assert!(
                matches!(ConfigFile::parse(&text), Err(IjiError::InvalidSpec(_))),
                "{name:?}"
            );
        }
    }

    #[test]
    fn rejects_repo_without_url() {
        let err = ConfigFile::parse("[repos.foo]\nversion = \"v1\"\n").unwrap_err();
        assert!(matches!(err, IjiError::TomlDe(_)));
    }

    #[test]
    fn rejects_repo_with_empty_url() {
        let err = ConfigFile::parse("[repos.foo]\nurl = \"  \"\n").unwrap_err();
        assert!(matches!(err, IjiError::InvalidConfig(_)));
    }

    #[test]
    fn allows_tables_in_any_order() {
        let file = ConfigFile::parse(
            r#"
            [paths]
            source_dir = "~/code"

            [repos.foo]
            url = "https://example.com/foo"
            "#,
        )
        .unwrap();

        assert_eq!(file.paths.source_dir.as_deref(), Some(Path::new("~/code")));
        assert_eq!(
            file.repos.get("foo").unwrap().url,
            "https://example.com/foo"
        );
    }

    #[test]
    fn rejects_unknown_fields() {
        for text in [
            "unknown = true\n",
            "[paths]\ndata_dir = \"/tmp\"\n",
            "[repos.foo]\nurl = \"https://example.com/r\"\nversoin = \"v1\"\n",
        ] {
            assert!(
                matches!(ConfigFile::parse(text), Err(IjiError::TomlDe(_))),
                "{text:?}"
            );
        }
    }

    #[test]
    fn rejects_empty_repo_options() {
        for field in ["version", "target"] {
            for value in ["", "   "] {
                let text = format!(
                    "[repos.foo]\nurl = \"https://example.com/r\"\n{field} = \"{value}\"\n"
                );
                assert!(
                    matches!(ConfigFile::parse(&text), Err(IjiError::InvalidConfig(_))),
                    "{field} = {value:?}"
                );
            }
        }
    }
}
