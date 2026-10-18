mod toml_store;

pub use toml_store::TomlStore;

use crate::{Result, package::InstalledPackage};

pub trait Database {
    fn all(&self) -> Result<Vec<InstalledPackage>>;

    fn get(&self, name: &str) -> Result<Option<InstalledPackage>>;

    fn insert(&mut self, pkg: InstalledPackage) -> Result<()>;

    fn remove(&mut self, name: &str) -> Result<Option<InstalledPackage>>;
}
