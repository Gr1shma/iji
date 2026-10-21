use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use iji::IjiError;
use iji::commands::{install, remove, update};
use iji::config::Config;
use iji::db::{Database, TomlStore};

fn git(dir: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

fn make_repo(root: &Path, name: &str) -> PathBuf {
    let repo = root.join(name);
    fs::create_dir_all(repo.join("src")).unwrap();
    fs::write(
        repo.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .unwrap();
    fs::write(repo.join("src/main.rs"), "fn main() {}\n").unwrap();

    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);
    repo
}

#[test]
fn install_then_remove_round_trip() {
    let root = std::env::temp_dir().join(format!("iji-flow-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let repo = make_repo(&root, "hello-iji");
    let cfg = Config::with_root(root.join("iji"));
    let url = format!("file://{}", repo.display());

    install::run(&cfg, std::slice::from_ref(&url), false).unwrap();
    let bin = cfg.bin_dir.join("hello-iji");
    assert!(bin.exists());
    let db = TomlStore::new(cfg.db_file());
    assert!(db.get("hello-iji").unwrap().is_some());

    // Reinstalling without --force is a skip, not an error.
    install::run(&cfg, &[url], false).unwrap();

    remove::run(&cfg, &["hello-iji".to_owned()]).unwrap();
    assert!(!bin.exists());
    assert!(db.get("hello-iji").unwrap().is_none());

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn bare_name_fails_without_touching_the_db() {
    let root = std::env::temp_dir().join(format!("iji-flow-bare-{}", std::process::id()));
    let cfg = Config::with_root(&root);

    let err = install::run(&cfg, &["foo".to_owned()], false).unwrap_err();

    assert!(matches!(err, IjiError::PackagesFailed(1)));
    assert!(TomlStore::new(cfg.db_file()).all().unwrap().is_empty());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn update_rebuilds_only_when_upstream_changes() {
    let root = std::env::temp_dir().join(format!("iji-flow-update-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let repo = make_repo(&root, "hello-iji");
    let cfg = Config::with_root(root.join("iji"));
    let url = format!("file://{}", repo.display());
    let db = TomlStore::new(cfg.db_file());

    install::run(&cfg, &[url], false).unwrap();
    let first = db.get("hello-iji").unwrap().unwrap().commit;
    assert!(first.is_some());

    update::run(&cfg, &[]).unwrap();
    assert_eq!(db.get("hello-iji").unwrap().unwrap().commit, first);

    fs::write(
        repo.join("src/main.rs"),
        "fn main() { println!(\"v2\"); }\n",
    )
    .unwrap();
    git(&repo, &["commit", "-q", "-a", "-m", "v2"]);
    update::run(&cfg, &[]).unwrap();
    assert_ne!(db.get("hello-iji").unwrap().unwrap().commit, first);

    fs::remove_dir_all(&root).unwrap();
}
