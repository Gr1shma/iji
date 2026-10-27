pub mod git;

pub use git::Git;

use std::path::Path;

use crate::Result;

pub trait Vcs {
    fn fetch(&self, url: &str, dest: &Path, version: Option<&str>) -> Result<()>;
    fn head(&self, repo: &Path) -> Result<String>;
    fn remote_head(&self, url: &str) -> Result<String>;
}
