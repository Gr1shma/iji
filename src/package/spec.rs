use std::str::FromStr;

use crate::IjiError;

#[derive(Debug, Clone, PartialEq)]
pub struct PackageSpec {
    pub name: String,
    pub version: Option<String>,
    pub target: Option<String>,
}

fn validate_name(name: &str) -> Result<(), IjiError> {
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

impl FromStr for PackageSpec {
    type Err = IjiError;
    // name@version,target
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(IjiError::InvalidSpec("empty package spec".into()));
        }

        let (rest, target) = match s.split_once(',') {
            Some((rest, target)) => {
                let target = target.trim();
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
            None => (s, None),
        };

        let (name, version) = match rest.split_once('@') {
            Some((name, version)) => {
                let version = version.trim();
                if version.is_empty() {
                    return Err(IjiError::InvalidSpec(format!("empty version in `{s}`")));
                }
                if version.contains('@') {
                    return Err(IjiError::InvalidSpec(format!("too many `@` in `{s}`")));
                }
                (name.trim(), Some(version.to_string()))
            }
            None => (rest.trim(), None),
        };

        validate_name(name)?;

        Ok(Self {
            name: name.to_string(),
            version,
            target,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<PackageSpec, IjiError> {
        s.parse()
    }

    #[test]
    fn parses_all_forms() {
        let p = parse("foo").unwrap();
        assert_eq!((p.name.as_str(), p.version, p.target), ("foo", None, None));

        let p = parse("foo@v1.2").unwrap();
        assert_eq!(p.version.as_deref(), Some("v1.2"));

        let p = parse("foo,bin").unwrap();
        assert_eq!(p.target.as_deref(), Some("bin"));

        let p = parse("foo@v1,bin").unwrap();
        assert_eq!(p.version.as_deref(), Some("v1"));
        assert_eq!(p.target.as_deref(), Some("bin"));
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
