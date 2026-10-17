use std::{path::Path, process::Command};

use crate::{IjiError, Result};

pub fn run(cmd: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| {
            IjiError::CommandFailed(format!("could not run `{cmd}` in {}: {e}", cwd.display()))
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(IjiError::CommandFailed(format!(
            "`{cmd} {}` in {} exited with {status}",
            args.join(" "),
            cwd.display(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_and_failure() {
        let dir = std::env::temp_dir();
        assert!(run("true", &[], &dir).is_ok());
        assert!(matches!(
            run("false", &[], &dir),
            Err(IjiError::CommandFailed(_))
        ));
    }

    #[test]
    fn missing_command_is_an_error() {
        let dir = std::env::temp_dir();
        assert!(matches!(
            run("iji-no-such-command", &[], &dir),
            Err(IjiError::CommandFailed(_))
        ));
    }
}
