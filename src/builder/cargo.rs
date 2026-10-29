use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::BuildSystem;
use crate::{IjiError, Result, util::process::run};

pub struct Cargo;

impl BuildSystem for Cargo {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn build(&self, src: &Path, _stage: &Path, target: Option<&str>) -> Result<()> {
        let mut args: Vec<&str> = vec!["build", "--release"];
        if let Some(bin) = target {
            args.extend(["--bin", bin]);
        }
        run("cargo", &args, src)
    }

    fn artifacts(&self, src: &Path, _stage: &Path, target: Option<&str>) -> Result<Vec<PathBuf>> {
        let bin_dir = src.join("target").join("release");
        let names = match target {
            Some(name) => vec![name.to_owned()],
            None => bin_names(src)?,
        };

        let mut out = Vec::with_capacity(names.len());
        for name in names {
            let exe = bin_dir.join(exe_name(&name));
            if !exe.is_file() {
                return Err(IjiError::NoBinaries(exe));
            }
            out.push(exe);
        }
        Ok(out)
    }
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

#[derive(Deserialize)]
struct Manifest {
    package: Option<Package>,
    bin: Option<Vec<BinEntry>>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    #[serde(default = "default_true")]
    autobins: bool,
}

#[derive(Deserialize)]
struct BinEntry {
    name: String,
    path: Option<String>,
}

fn default_true() -> bool {
    true
}

fn bin_names(src: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(src.join("Cargo.toml"))?;
    let m: Manifest = toml::from_str(&text)?;
    let pkg = m.package.ok_or_else(|| {
        IjiError::InvalidSpec(format!(
            "{} is a workspace root; pick a binary with `name,target`",
            src.display()
        ))
    })?;

    let explicit = m.bin.unwrap_or_default();
    let mut names: Vec<String> = explicit.iter().map(|b| b.name.clone()).collect();

    let claimed: HashSet<PathBuf> = explicit
        .iter()
        .filter_map(|b| b.path.as_ref())
        .map(|p| src.join(p))
        .collect();

    if pkg.autobins {
        let main_rs = src.join("src/main.rs");
        if main_rs.is_file() && !claimed.contains(&main_rs) {
            names.push(pkg.name.clone());
        }
        if let Ok(entries) = fs::read_dir(src.join("src/bin")) {
            for path in entries.flatten().map(|e| e.path()) {
                let (name, bin_path) = if path.is_dir() && path.join("main.rs").is_file() {
                    (path.file_name(), path.join("main.rs"))
                } else if path.extension().is_some_and(|x| x == "rs") {
                    (path.file_stem(), path.clone())
                } else {
                    continue;
                };
                if claimed.contains(&bin_path) {
                    continue;
                }
                if let Some(n) = name.and_then(|n| n.to_str()) {
                    names.push(n.to_owned());
                }
            }
        }
    }

    names.sort();
    names.dedup();
    if names.is_empty() {
        return Err(IjiError::InvalidSpec(format!(
            "no binaries found in {} (library-only crate?)",
            src.display()
        )));
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    #[test]
    fn discovers_bins() {
        let root = temp_dir("iji-bins");

        write(&root, "Cargo.toml", "[package]\nname = \"foo\"\n");
        write(&root, "src/main.rs", "");
        write(&root, "src/bin/bar.rs", "");
        assert_eq!(bin_names(&root).unwrap(), ["bar", "foo"]);

        write(
            &root,
            "Cargo.toml",
            "[package]\nname = \"foo\"\nautobins = false\n[[bin]]\nname = \"only\"\n",
        );
        assert_eq!(bin_names(&root).unwrap(), ["only"]);

        write(&root, "Cargo.toml", "[workspace]\nmembers = []\n");
        assert!(matches!(bin_names(&root), Err(IjiError::InvalidSpec(_))));

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn explicit_bin_path_suppresses_implicit_main() {
        let root = temp_dir("iji-bins-claim-main");
        write(
            &root,
            "Cargo.toml",
            r#"
[package]
name = "mypkg"

[[bin]]
name = "mytool"
path = "src/main.rs"
"#,
        );
        write(&root, "src/main.rs", "");
        assert_eq!(bin_names(&root).unwrap(), ["mytool"]);
        fs::remove_dir_all(&root).unwrap();
    }
}
