use std::{
    env,
    ffi::OsString,
    path::{self, Path, PathBuf},
};

use crate::{IjiError, Result};

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

fn absolute(var: &str, p: PathBuf) -> Result<PathBuf> {
    let p = expand_tilde(p);
    if p.is_relative() {
        return Err(IjiError::RelativePath {
            var: var.to_owned(),
            path: p,
        });
    }
    path::absolute(&p).map_err(|source| IjiError::PathResolve { path: p, source })
}

fn resolve_with(
    env_key: &str,
    env_value: Option<OsString>,
    fallback: Option<PathBuf>,
) -> Result<PathBuf> {
    match non_empty_path(env_value) {
        Some(p) => absolute(env_key, p),
        None => fallback.ok_or(IjiError::NoHomeDir),
    }
}

fn resolve_file_with(
    env_key: &str,
    file_key: &str,
    env_value: Option<OsString>,
    file_value: Option<PathBuf>,
    fallback: Option<PathBuf>,
) -> Result<PathBuf> {
    match non_empty_path(env_value) {
        Some(p) => absolute(env_key, p),
        None => match non_empty_path(file_value.map(PathBuf::into_os_string)) {
            Some(p) => absolute(file_key, p),
            None => fallback.ok_or(IjiError::NoHomeDir),
        },
    }
}

pub(super) fn resolve(env_key: &str, fallback: Option<PathBuf>) -> Result<PathBuf> {
    resolve_with(env_key, env::var_os(env_key), fallback)
}

pub(super) fn resolve_file(
    env_key: &str,
    file_key: &str,
    file_value: Option<PathBuf>,
    fallback: Option<PathBuf>,
) -> Result<PathBuf> {
    resolve_file_with(
        env_key,
        file_key,
        env::var_os(env_key),
        file_value,
        fallback,
    )
}

pub(super) fn path_var_contains(path_var: Option<OsString>, dir: &Path) -> bool {
    path_var.is_some_and(|v| env::split_paths(&v).any(|p| p == dir))
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
    fn file_value_is_used_when_env_is_unset() {
        let got = resolve_file_with(
            "IJI_X",
            "paths.x",
            None,
            Some("/from-file".into()),
            Some("/fallback".into()),
        )
        .unwrap();
        assert_eq!(got, PathBuf::from("/from-file"));
    }

    #[test]
    fn env_wins_over_file_value() {
        let got = resolve_file_with(
            "IJI_X",
            "paths.x",
            os("/from-env"),
            Some("/from-file".into()),
            Some("/fallback".into()),
        )
        .unwrap();
        assert_eq!(got, PathBuf::from("/from-env"));
    }

    #[test]
    fn fallback_is_used_when_env_and_file_are_empty() {
        let got = resolve_file_with(
            "IJI_X",
            "paths.x",
            os(""),
            Some(PathBuf::new()),
            Some("/fallback".into()),
        )
        .unwrap();
        assert_eq!(got, PathBuf::from("/fallback"));
    }

    #[test]
    fn no_env_file_or_fallback_errors() {
        let err = resolve_file_with("IJI_X", "paths.x", None, None, None).unwrap_err();
        assert!(matches!(err, IjiError::NoHomeDir));
    }

    #[test]
    fn relative_file_value_is_rejected() {
        let err =
            resolve_file_with("IJI_X", "paths.x", None, Some("bin".into()), None).unwrap_err();
        assert!(matches!(err, IjiError::RelativePath { var, .. } if var == "paths.x"));
    }

    #[test]
    fn tilde_in_file_value_is_expanded_to_home() {
        if let Some(home) = dirs::home_dir() {
            let got =
                resolve_file_with("IJI_X", "paths.x", None, Some("~/bin".into()), None).unwrap();
            assert_eq!(got, home.join("bin"));
        }
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
    fn path_var_detection() {
        let v = env::join_paths(["/usr/bin", "/r/bin"]).unwrap();
        assert!(path_var_contains(Some(v.clone()), Path::new("/r/bin")));
        assert!(!path_var_contains(Some(v), Path::new("/other")));
        assert!(!path_var_contains(None, Path::new("/r/bin")));
    }
}
