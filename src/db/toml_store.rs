use std::{fs, io::ErrorKind, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::Database;
use crate::{Result, package::InstalledPackage};

#[derive(Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    package: Vec<InstalledPackage>,
}

pub struct TomlStore {
    path: PathBuf,
}

impl TomlStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn read(&self) -> Result<File> {
        match fs::read_to_string(&self.path) {
            Ok(text) => Ok(toml::from_str(&text)?),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(File::default()),
            Err(e) => Err(e.into()),
        }
    }

    fn write(&self, mut file: File) -> Result<()> {
        file.package.sort_by(|a, b| a.name.cmp(&b.name));
        let text = toml::to_string_pretty(&file)?;
        let tmp = self.path.with_extension("toml.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path).inspect_err(|_| {
            let _ = fs::remove_file(&tmp);
        })?;
        Ok(())
    }
}

impl Database for TomlStore {
    fn all(&self) -> Result<Vec<InstalledPackage>> {
        Ok(self.read()?.package)
    }

    fn get(&self, name: &str) -> Result<Option<InstalledPackage>> {
        Ok(self.read()?.package.into_iter().find(|p| p.name == name))
    }

    fn insert(&mut self, pkg: InstalledPackage) -> Result<()> {
        let mut file = self.read()?;
        match file.package.iter_mut().find(|p| p.name == pkg.name) {
            Some(existing) => *existing = pkg,
            None => file.package.push(pkg),
        }
        self.write(file)
    }

    fn remove(&mut self, name: &str) -> Result<Option<InstalledPackage>> {
        let mut file = self.read()?;
        let Some(pos) = file.package.iter().position(|p| p.name == name) else {
            return Ok(None);
        };
        let removed = file.package.remove(pos);
        self.write(file)?;
        Ok(Some(removed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str, version: Option<&str>) -> InstalledPackage {
        InstalledPackage {
            name: name.into(),
            url: format!("https://example.com/{name}"),
            version: version.map(Into::into),
            target: None,
            files: vec![PathBuf::from(format!("/bin/{name}"))],
        }
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("iji-db-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn insert_get_replace_remove() {
        let dir = temp_dir("crud");
        let mut db = TomlStore::new(dir.join("installed.toml"));

        // missing file is empty, not an error
        assert!(db.all().unwrap().is_empty());
        assert!(db.get("a").unwrap().is_none());

        db.insert(pkg("a", None)).unwrap();
        db.insert(pkg("b", Some("v1"))).unwrap();
        assert_eq!(db.get("b").unwrap(), Some(pkg("b", Some("v1"))));

        // upsert replaces instead of duplicating
        db.insert(pkg("a", Some("v2"))).unwrap();
        assert_eq!(db.all().unwrap().len(), 2);
        assert_eq!(db.get("a").unwrap().unwrap().version.as_deref(), Some("v2"));

        // remove returns the record and deletes it
        assert_eq!(db.remove("a").unwrap().unwrap().name, "a");
        assert!(db.get("a").unwrap().is_none());
        assert!(db.remove("a").unwrap().is_none());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn writes_sorted_and_leaves_no_temp_file() {
        let dir = temp_dir("sorted");
        let path = dir.join("installed.toml");
        let mut db = TomlStore::new(path.clone());

        db.insert(pkg("zeta", None)).unwrap();
        db.insert(pkg("alpha", None)).unwrap();

        let names: Vec<_> = db.all().unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["alpha", "zeta"]);
        assert!(!path.with_extension("toml.tmp").exists());

        fs::remove_dir_all(&dir).unwrap();
    }
}
