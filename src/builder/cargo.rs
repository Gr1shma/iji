use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::BuildSystem;
use crate::util::process::{capture, run};
use crate::{IjiError, Result};

pub struct Cargo;

impl BuildSystem for Cargo {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn build(&self, src: &Path, _stage: &Path, target: Option<&str>) -> Result<()> {
        run("cargo", &build_args(target), src)
    }

    fn artifacts(&self, src: &Path, _stage: &Path, target: Option<&str>) -> Result<Vec<PathBuf>> {
        let mut args = build_args(target);
        args.push("--message-format=json");
        let exes = executables(&capture("cargo", &args, src)?);
        if exes.is_empty() {
            return Err(IjiError::NoBinaries(src.to_path_buf()));
        }
        Ok(exes)
    }
}

fn build_args(target: Option<&str>) -> Vec<&str> {
    let mut args = vec!["build", "--release"];
    if let Some(bin) = target {
        args.extend(["--bin", bin]);
    }
    args
}

#[derive(Deserialize)]
struct Message {
    reason: String,
    target: Option<MessageTarget>,
    executable: Option<PathBuf>,
}

#[derive(Deserialize)]
struct MessageTarget {
    kind: Vec<String>,
}

fn executables(stdout: &str) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Message>(line).ok())
        .filter(|m| m.reason == "compiler-artifact")
        .filter(|m| {
            m.target
                .as_ref()
                .is_some_and(|t| t.kind.iter().any(|k| k == "bin"))
        })
        .filter_map(|m| m.executable)
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_only_bin_executables() {
        let out = [
            r#"{"reason":"compiler-artifact","target":{"kind":["lib"]},"executable":null}"#,
            r#"{"reason":"compiler-artifact","target":{"kind":["custom-build"]},"executable":null}"#,
            r#"{"reason":"compiler-artifact","target":{"kind":["bin"]},"executable":"/t/release/b"}"#,
            r#"{"reason":"compiler-artifact","target":{"kind":["bin"]},"executable":"/t/release/a"}"#,
            "not json",
            r#"{"reason":"build-finished","success":true}"#,
        ]
        .join("\n");

        assert_eq!(
            executables(&out),
            [PathBuf::from("/t/release/a"), PathBuf::from("/t/release/b")]
        );
    }
}
