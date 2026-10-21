use crate::Result;
use crate::config::Config;
use crate::db::{Database, TomlStore};
use crate::util::log;

pub fn run(cfg: &Config) -> Result<()> {
    let db = TomlStore::new(cfg.db_file());
    let packages = db.all()?;

    if packages.is_empty() {
        log::info("no packages installed");
        return Ok(());
    }

    for p in &packages {
        let version = p.version.as_deref().unwrap_or("default");
        println!("{} {} {}", p.name, version, p.url);
    }
    Ok(())
}
