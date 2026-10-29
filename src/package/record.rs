use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub url: String,
    pub version: Option<String>,
    pub target: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    pub files: Vec<PathBuf>,
    #[serde(default)]
    pub custom: bool,
}
