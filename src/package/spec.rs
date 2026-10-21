use std::str::FromStr;

use crate::{IjiError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSpec {
    pub name: String,
    pub url: Option<String>,
    pub version: Option<String>,
    pub target: Option<String>,
}

pub fn validate_name(name: &str) -> Result<()> {
    let bad = name.is_empty()
        || name.starts_with('.')
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if bad {
        return Err(IjiError::InvalidSpec(format!("invalid name `{name}`")));
    }
    Ok(())
}

fn is_url(s: &str) -> bool {
    s.contains("://") || s.starts_with("git@")
}

fn name_from_url(url: &str) -> &str {
    let seg = url
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or_default();
    seg.strip_suffix(".git").unwrap_or(seg)
}

fn split_suffix(s: &str, start: usize) -> Result<(&str, Option<String>, Option<String>)> {
    let tail = &s[start..];

    let (rest, target) = match tail.split_once(',') {
        Some((rest, target)) => {
            if target.is_empty() {
                return Err(IjiError::InvalidSpec(format!("empty target in `{s}`")));
            }
            if target.contains([',', '@']) {
                return Err(IjiError::InvalidSpec(format!(
                    "unexpected `,` or `@` in target; use name@version,target (`{s}`)"
                )));
            }
            (rest, Some(target.to_owned()))
        }
        None => (tail, None),
    };

    let (name, version) = match rest.split_once('@') {
        Some((name, version)) => {
            if version.is_empty() {
                return Err(IjiError::InvalidSpec(format!("empty version in `{s}`")));
            }
            if version.contains('@') {
                return Err(IjiError::InvalidSpec(format!("too many `@` in `{s}`")));
            }
            (name, Some(version.to_owned()))
        }
        None => (rest, None),
    };

    Ok((&s[..start + name.len()], version, target))
}

impl FromStr for PackageSpec {
    type Err = IjiError;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Err(IjiError::InvalidSpec("empty package spec".into()));
        }

        if is_url(s) {
            let start = s.rfind(['/', ':']).map_or(0, |i| i + 1);
            let (url, version, target) = split_suffix(s, start)?;
            let name = name_from_url(url);
            validate_name(name)?;
            return Ok(Self {
                name: name.to_owned(),
                url: Some(url.to_owned()),
                version,
                target,
            });
        }

        let (name, version, target) = split_suffix(s, 0)?;
        validate_name(name)?;
        Ok(Self {
            name: name.to_owned(),
            url: None,
            version,
            target,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<PackageSpec> {
        s.parse()
    }

    #[test]
    fn parses_all_forms() {
        let p = parse("foo").unwrap();
        assert_eq!(
            (p.name.as_str(), p.url, p.version, p.target),
            ("foo", None, None, None)
        );

        let p = parse("foo@v1,bin").unwrap();
        assert_eq!(p.version.as_deref(), Some("v1"));
        assert_eq!(p.target.as_deref(), Some("bin"));
    }

    #[test]
    fn parses_urls() {
        let p = parse("https://github.com/u/r.git").unwrap();
        assert_eq!(p.name, "r");
        assert_eq!(p.url.as_deref(), Some("https://github.com/u/r.git"));

        let p = parse("git@github.com:u/r@v1,bin").unwrap();
        assert_eq!(p.name, "r");
        assert_eq!(p.url.as_deref(), Some("git@github.com:u/r"));
        assert_eq!(p.version.as_deref(), Some("v1"));
        assert_eq!(p.target.as_deref(), Some("bin"));
    }

    #[test]
    fn rejects_url_with_invalid_derived_name() {
        for s in [
            "https://github.com/u/.git",
            "https://github.com/u/..",
            "https://",
        ] {
            assert!(matches!(parse(s), Err(IjiError::InvalidSpec(_))), "{s:?}");
        }
    }

    #[test]
    fn rejects_empty_parts() {
        for s in ["", "@v1", ",bin", "foo@", "foo,", "foo@v1,"] {
            assert!(matches!(parse(s), Err(IjiError::InvalidSpec(_))), "{s:?}");
        }
    }

    #[test]
    fn rejects_duplicate_or_misordered_markers() {
        for s in ["foo@v1@v2", "foo,a,b", "foo,bin@v1"] {
            assert!(matches!(parse(s), Err(IjiError::InvalidSpec(_))), "{s:?}");
        }
    }

    #[test]
    fn rejects_path_traversal() {
        for s in [".", "..", ".hidden", "a/b", "../foo", "a\\b"] {
            assert!(matches!(parse(s), Err(IjiError::InvalidSpec(_))), "{s:?}");
        }
    }
}
