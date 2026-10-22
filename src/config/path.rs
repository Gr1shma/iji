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

pub(super) fn resolve(env_key: &str, fallback: Option<PathBuf>) -> Result<PathBuf> {
    resolve_with(env_key, env::var_os(env_key), fallback)
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
