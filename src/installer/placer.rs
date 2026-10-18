use std::fs;
use std::path::{Path, PathBuf};

use crate::{IjiError, Result};

pub fn place(artifacts: &[PathBuf], bin_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut placed = Vec::with_capacity(artifacts.len());

    for art in artifacts {
        let name = art.file_name().ok_or_else(|| {
            IjiError::CommandFailed(format!("artifact `{}` has no file name", art.display()))
        })?;
        let dest = bin_dir.join(name);

        let mut tmp_name = std::ffi::OsString::from(".");
        tmp_name.push(name);
        tmp_name.push(".iji-tmp");
        let tmp = bin_dir.join(tmp_name);

        let result = fs::copy(art, &tmp).and_then(|_| fs::rename(&tmp, &dest));
        if let Err(e) = result {
            let _ = fs::remove_file(&tmp);
            return Err(e.into());
        }

        placed.push(dest);
    }

    Ok(placed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    #[test]
    fn places_and_overwrites() {
        let root = std::env::temp_dir().join(format!("iji-place-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (src, bin) = (root.join("src"), root.join("bin"));
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&bin).unwrap();

        let art = src.join("tool");
        fs::write(&art, "v1").unwrap();
        fs::set_permissions(&art, fs::Permissions::from_mode(0o755)).unwrap();

        let placed = place(std::slice::from_ref(&art), &bin).unwrap();
        assert_eq!(placed, [bin.join("tool")]);
        assert_eq!(fs::read_to_string(&placed[0]).unwrap(), "v1");
        assert_eq!(
            fs::metadata(&placed[0]).unwrap().permissions().mode() & 0o111,
            0o111
        );

        fs::write(&art, "v2").unwrap();
        place(&[art], &bin).unwrap();
        assert_eq!(fs::read_to_string(bin.join("tool")).unwrap(), "v2");

        fs::remove_dir_all(&root).unwrap();
    }
}
