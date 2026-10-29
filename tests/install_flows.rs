use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use iji::IjiError;
use iji::commands::{install, remove, update};
use iji::config::{Config, Repo};
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

fn make_bins_repo(root: &Path, name: &str, bins: &[&str]) -> PathBuf {
    let repo = root.join(name);
    fs::create_dir_all(repo.join("src/bin")).unwrap();
    fs::write(
        repo.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .unwrap();
    for bin in bins {
        fs::write(repo.join(format!("src/bin/{bin}.rs")), "fn main() {}\n").unwrap();
    }

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
fn force_reinstall_removes_stale_files() {
    let root = std::env::temp_dir().join(format!("iji-flow-force-stale-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let repo = make_bins_repo(&root, "two-bins", &["a", "b"]);
    let cfg = Config::with_root(root.join("iji"));
    let url = format!("file://{}", repo.display());
    let db = TomlStore::new(cfg.db_file());

    install::run(&cfg, std::slice::from_ref(&url), false).unwrap();
    assert!(cfg.bin_dir.join("a").exists());
    assert!(cfg.bin_dir.join("b").exists());

    fs::remove_file(repo.join("src/bin/b.rs")).unwrap();
    git(&repo, &["commit", "-q", "-a", "-m", "drop b"]);

    install::run(&cfg, &[url], true).unwrap();
    assert!(cfg.bin_dir.join("a").exists());
    assert!(!cfg.bin_dir.join("b").exists());

    let installed = db.get("two-bins").unwrap().unwrap();
    assert_eq!(installed.files, [cfg.bin_dir.join("a")]);

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
fn bare_name_installs_from_configured_repo() {
    let root = std::env::temp_dir().join(format!("iji-flow-repo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let repo = make_repo(&root, "hello-iji");
    let mut cfg = Config::with_root(root.join("iji"));
    cfg.repos.insert(
        "hello-iji".to_owned(),
        Repo {
            url: format!("file://{}", repo.display()),
            version: None,
            target: None,
            run: None,
            script: None,
        },
    );

    install::run(&cfg, &["hello-iji".to_owned()], false).unwrap();
    assert!(cfg.bin_dir.join("hello-iji").exists());

    let db = TomlStore::new(cfg.db_file());
    let installed = db.get("hello-iji").unwrap().unwrap();
    assert_eq!(installed.url, format!("file://{}", repo.display()));

    // Updates and removals use the recorded URL, so the config can disappear.
    cfg.repos.clear();
    update::run(&cfg, &["hello-iji".to_owned()]).unwrap();
    remove::run(&cfg, &["hello-iji".to_owned()]).unwrap();
    assert!(db.get("hello-iji").unwrap().is_none());

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn repo_version_pins_updates() {
    let root = std::env::temp_dir().join(format!("iji-flow-pinned-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let repo = make_repo(&root, "hello-iji");
    git(&repo, &["tag", "v1"]);
    fs::write(
        repo.join("src/main.rs"),
        "fn main() { println!(\"newer\"); }\n",
    )
    .unwrap();
    git(&repo, &["commit", "-q", "-a", "-m", "newer"]);

    let mut cfg = Config::with_root(root.join("iji"));
    cfg.repos.insert(
        "hello-iji".to_owned(),
        Repo {
            url: format!("file://{}", repo.display()),
            version: Some("v1".into()),
            target: None,
            run: None,
            script: None,
        },
    );

    install::run(&cfg, &["hello-iji".to_owned()], false).unwrap();
    let db = TomlStore::new(cfg.db_file());
    let first = db.get("hello-iji").unwrap().unwrap().commit;

    update::run(&cfg, &[]).unwrap();
    assert_eq!(db.get("hello-iji").unwrap().unwrap().commit, first);

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn config_file_drives_repo_lookup_and_paths() {
    let root = std::env::temp_dir().join(format!("iji-flow-config-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let repo = make_repo(&root, "hello-iji");
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("iji.toml"),
        format!(
            "[repos.hello-iji]\nurl = \"file://{}\"\n\n[paths]\nbin_dir = \"{}/custom-bin\"\ncache_dir = \"{}/custom-cache\"\nsource_dir = \"{}/custom-src\"\n",
            repo.display(),
            root.display(),
            root.display(),
            root.display()
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_iji"))
        .args(["--quiet", "install", "hello-iji"])
        .env("IJI_CONFIG_DIR", &config_dir)
        .env("IJI_DATA_DIR", root.join("data"))
        .env_remove("IJI_BIN_DIR")
        .env_remove("IJI_CACHE_DIR")
        .env_remove("IJI_SOURCE_DIR")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(root.join("custom-bin/hello-iji").exists());
    assert!(root.join("custom-src/hello-iji").is_dir());
    assert!(!root.join("custom-cache/src/hello-iji").exists());
    assert!(root.join("custom-cache").is_dir());
    assert!(root.join("data/installed.toml").is_file());

    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn relative_path_in_config_is_rejected() {
    let root = std::env::temp_dir().join(format!("iji-flow-relpath-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).unwrap();

    for (key, value) in [("bin_dir", "bin"), ("source_dir", "srcs")] {
        fs::write(
            config_dir.join("iji.toml"),
            format!("[paths]\n{key} = \"{value}\"\n"),
        )
        .unwrap();

        let output = Command::new(env!("CARGO_BIN_EXE_iji"))
            .arg("list")
            .env("IJI_CONFIG_DIR", &config_dir)
            .env("IJI_DATA_DIR", root.join("data"))
            .env_remove("IJI_BIN_DIR")
            .env_remove("IJI_CACHE_DIR")
            .env_remove("IJI_SOURCE_DIR")
            .output()
            .unwrap();

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "{key}");
        assert!(stderr.contains(&format!("paths.{key}")), "{stderr}");
    }

    fs::remove_dir_all(&root).unwrap();
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
