use std::{fs, path::Path, process::Command};

use super::Vcs;
use crate::{IjiError, Result, util::process::run};

pub struct Git;

impl Vcs for Git {
    fn fetch(&self, url: &str, dest: &Path, version: Option<&str>) -> Result<()> {
        let dest_str = dest.to_str().ok_or_else(|| {
            IjiError::CommandFailed(format!("path is not valid UTF-8: {}", dest.display()))
        })?;

        let parent = dest
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(|source| IjiError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;

        let mut args = vec!["clone", "--depth", "1"];
        if let Some(v) = version {
            args.extend(["--branch", v]);
        }
        args.extend(["--", url, dest_str]);

        run("git", &args, parent)
    }
    fn head(&self, repo: &Path) -> Result<String> {
        let out = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo)
            .output()
            .map_err(|e| {
                IjiError::CommandFailed(format!("could not run `git` in {}: {e}", repo.display()))
            })?;

        if !out.status.success() {
            return Err(IjiError::CommandFailed(format!(
                "`git rev-parse HEAD` in {} exited with {}",
                repo.display(),
                out.status
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        run("git", args, dir).unwrap();
    }

    fn commit(dir: &Path, msg: &str) {
        let args = [
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-q",
            "-a",
            "-m",
            msg,
        ];
        git(dir, &args);
    }

    #[test]
    fn fetches_default_branch_and_tag() {
        let root = std::env::temp_dir().join(format!("iji-git-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let origin = root.join("origin");
        fs::create_dir_all(&origin).unwrap();

        // origin: commit "1" tagged v1, then commit "2" on top
        git(&origin, &["init", "-q"]);
        fs::write(origin.join("a.txt"), "1").unwrap();
        git(&origin, &["add", "."]);
        commit(&origin, "one");
        git(&origin, &["tag", "v1"]);
        fs::write(origin.join("a.txt"), "2").unwrap();
        commit(&origin, "two");

        let url = origin.to_str().unwrap();

        Git.fetch(url, &root.join("head"), None).unwrap();
        assert_eq!(fs::read_to_string(root.join("head/a.txt")).unwrap(), "2");

        Git.fetch(url, &root.join("tagged"), Some("v1")).unwrap();
        assert_eq!(fs::read_to_string(root.join("tagged/a.txt")).unwrap(), "1");

        assert!(Git.fetch(url, &root.join("bad"), Some("nope")).is_err());

        let head = Git.head(&root.join("head")).unwrap();
        let tagged = Git.head(&root.join("tagged")).unwrap();
        assert_eq!(head.len(), 40);
        assert!(head.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(head, tagged);

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn head_fails_outside_a_repo() {
        let dir = std::env::temp_dir().join(format!("iji-git-nohead-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        assert!(matches!(Git.head(&dir), Err(IjiError::CommandFailed(_))));

        fs::remove_dir_all(&dir).unwrap();
    }
}
