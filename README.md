# iji

iji is a small package manager for programs that live in git repos. Point it at a repo and it will clone, build, and install the program for you.

> **Note:** iji builds and runs code from repos you choose. Only install things you trust.

## Install

You'll need Rust and Cargo.

```sh
git clone https://github.com/Gr1shma/iji
cd iji
cargo install --path .
```

iji puts package binaries in `~/.local/bin`. If that folder isn't in your `PATH` yet, add it:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

The general shape of a command is:

```text
iji [options] <command> [packages...]
```

Here are the commands:

- `install` (or `i`): clone, build, and install packages
- `list` (or `l`): show what's installed
- `update` (or `u`): update packages. With no names, it updates everything.
- `remove` (or `r`): uninstall packages

These options work with every command:

- `-q`, `--quiet`: only print warnings and errors
- `-v`, `--verbose`: print git and build output too

`install` also takes `-f`, `--force`: reinstall even when the package is already there, and skip the check for files owned by another package.

## Installing a package

For now, you need to give iji the repo URL. Bare package names aren't supported yet.

```sh
iji install https://github.com/u/r
```

You can also choose a version or a target:

```text
<git-url>[@version][,target]
```

```sh
iji install https://github.com/u/r@v1.2
iji install https://github.com/u/r@v1.2,mytool
```

The package name comes from the end of the URL, with `.git` removed. Add `@version` to install a tag or branch. Packages installed this way stay on that version, so `update` leaves them alone.

To update or remove a package, use its name (the last part of the URL):

```sh
iji update r
iji remove r
```

A target is useful when a Cargo repo builds more than one binary. It tells iji which one you want.

## What works

iji is still pretty early. Right now it only builds Cargo projects and only runs on Unix.

When you run `update`, iji checks the latest commit and rebuilds only if something changed. If the package is pinned to a version, it gets skipped.

Known limitations:

- `CARGO_TARGET_DIR` and a custom `target-dir` in `.cargo/config` aren't honored.
- `install --force` leaves behind files an older version installed that the new one doesn't (`update` cleans these up).

## Configuration

Most people won't need a config file. If you do, these environment variables let you move things around:

| Variable | What it does |
| --- | --- |
| `IJI_BIN_DIR` | Where package binaries are installed |
| `IJI_DATA_DIR` | Where `installed.toml` is stored |
| `IJI_CACHE_DIR` | Where sources are cloned and built |
| `IJI_CONFIG_DIR` | Reserved for `iji.toml` |

Set `NO_COLOR` if you don't want color in the output.

## Development

Run these before sending a patch:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

## License

iji is released under the [MIT license](LICENSE).

