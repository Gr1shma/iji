use std::ffi::OsString;
use std::fmt::Display;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Quiet,
    Normal,
    Verbose,
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Debug,
    Info,
    Success,
    Warn,
    Error,
}

static VERBOSITY: AtomicU8 = AtomicU8::new(Verbosity::Normal as u8);

pub fn set_verbosity(v: Verbosity) {
    VERBOSITY.store(v as u8, Ordering::Relaxed);
}

pub fn verbosity() -> Verbosity {
    match VERBOSITY.load(Ordering::Relaxed) {
        0 => Verbosity::Quiet,
        2 => Verbosity::Verbose,
        _ => Verbosity::Normal,
    }
}

fn visible(v: Verbosity, kind: Kind) -> bool {
    match kind {
        Kind::Warn | Kind::Error => true,
        Kind::Info | Kind::Success => v != Verbosity::Quiet,
        Kind::Debug => v == Verbosity::Verbose,
    }
}

fn use_color(is_tty: bool, no_color: Option<OsString>) -> bool {
    is_tty && no_color.is_none_or(|v| v.is_empty())
}

fn write_line(mut out: impl Write, color: bool, tag: &str, code: &str, msg: &dyn Display) {
    let _ = if color {
        writeln!(out, "\x1b[{code}m[{tag}]\x1b[0m {msg}")
    } else {
        writeln!(out, "[{tag}] {msg}")
    };
}

fn emit(kind: Kind, msg: impl Display) {
    if !visible(verbosity(), kind) {
        return;
    }
    let (tag, code) = match kind {
        Kind::Debug => ("debug", "2"),
        Kind::Info => ("info", "36"),
        Kind::Success => ("ok", "32"),
        Kind::Warn => ("warning", "33"),
        Kind::Error => ("error", "1;31"),
    };
    let no_color = std::env::var_os("NO_COLOR");
    match kind {
        Kind::Info | Kind::Success => {
            let out = io::stdout();
            let color = use_color(out.is_terminal(), no_color);
            write_line(out, color, tag, code, &msg);
        }
        _ => {
            let err = io::stderr();
            let color = use_color(err.is_terminal(), no_color);
            write_line(err, color, tag, code, &msg);
        }
    }
}

pub fn info(msg: impl Display) {
    emit(Kind::Info, msg);
}

pub fn success(msg: impl Display) {
    emit(Kind::Success, msg);
}

pub fn debug(msg: impl Display) {
    emit(Kind::Debug, msg);
}

pub fn warn(msg: impl Display) {
    emit(Kind::Warn, msg);
}

pub fn error(msg: impl Display) {
    emit(Kind::Error, msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings_and_errors_are_never_hidden() {
        for v in [Verbosity::Quiet, Verbosity::Normal, Verbosity::Verbose] {
            assert!(visible(v, Kind::Warn));
            assert!(visible(v, Kind::Error));
        }
    }

    #[test]
    fn quiet_hides_info_and_verbose_shows_debug() {
        assert!(!visible(Verbosity::Quiet, Kind::Info));
        assert!(!visible(Verbosity::Quiet, Kind::Success));
        assert!(visible(Verbosity::Normal, Kind::Info));
        assert!(!visible(Verbosity::Normal, Kind::Debug));
        assert!(visible(Verbosity::Verbose, Kind::Debug));
    }

    #[test]
    fn color_requires_tty_and_respects_no_color() {
        assert!(use_color(true, None));
        assert!(use_color(true, Some("".into())));
        assert!(!use_color(true, Some("1".into())));
        assert!(!use_color(false, None));
    }
}
