use std::{path::Path, process::Command};

use crate::util::log::{self, Verbosity};
use crate::{IjiError, Result};

const TAIL_LINES: usize = 20;

pub fn run(cmd: &str, args: &[&str], cwd: &Path) -> Result<()> {
    let spawn_err = |e: std::io::Error| {
        IjiError::CommandFailed(format!("could not run `{cmd}` in {}: {e}", cwd.display()))
    };
    let exit_err = |status: std::process::ExitStatus, stderr: &str| {
        let mut msg = format!(
            "`{cmd} {}` in {} exited with {status}",
            args.join(" "),
            cwd.display(),
        );
        if !stderr.trim().is_empty() {
            msg.push('\n');
            msg.push_str(&tail(stderr, TAIL_LINES));
        }
        IjiError::CommandFailed(msg)
    };

    let mut command = Command::new(cmd);
    command.args(args).current_dir(cwd);

    if log::verbosity() == Verbosity::Verbose {
        let status = command.status().map_err(spawn_err)?;
        return if status.success() {
            Ok(())
        } else {
            Err(exit_err(status, ""))
        };
    }

    let out = command.output().map_err(spawn_err)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(exit_err(out.status, &String::from_utf8_lossy(&out.stderr)))
    }
}

fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
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

    #[test]
    fn tail_keeps_only_the_last_lines() {
        assert_eq!(tail("a\nb\nc\nd", 2), "c\nd");
        assert_eq!(tail("a", 5), "a");
    }
}
