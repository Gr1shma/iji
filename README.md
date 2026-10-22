# iji

iji is a small package manager for programs that live in git repos. Point it at a repo and it will clone, build, and install the program for you.

> **Note:** iji builds and runs code from repos you choose. Only install things you trust.

## Install

From crates.io (needs Rust and Cargo):

```sh
cargo install iji
```

Or from source:

```sh
git clone https://github.com/Gr1shma/iji
cd iji
cargo install --path .
```

Building packages also needs the tool for whichever build system the repo uses (`cargo`, `go`, `cmake`, or `make`).

iji puts package binaries in `~/.local/bin`. If that folder isn't in your `PATH` yet, add it:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

`cargo install` puts iji itself in `~/.cargo/bin`, which is separate from the directory where iji installs packages.

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

A target is useful when a repo builds more than one thing. What it means depends on the build system:

| Build system | `target` |
| --- | --- |
| Cargo | Binary name (`--bin`) |
| Go | Package path, such as `cmd/foo` (defaults to `./...`) |
| CMake | Build target (`--target`); the install step still runs afterward |
| Make | Goal for the first `make` run; `make install` still runs afterward |

For example, to build only the `cmd/foo` package in a Go repo:

```sh
iji install https://github.com/u/r@v1.2,cmd/foo
```

## What works

iji is still pretty early, and it only runs on Unix. It detects the build system from the repo:

| Build system | Detected by | Tool needed |
| --- | --- | --- |
| Cargo | `Cargo.toml` | `cargo` |
| Go | `go.mod` | `go` |
| CMake | `CMakeLists.txt` | `cmake` |
| Make | `GNUmakefile`, `makefile`, or `Makefile` | `make` |

Detection happens in that order. Go, CMake, and Make builds stage their executables in `usr/bin`, and iji copies those into your bin directory. Files a project installs elsewhere (libraries, data, man pages) aren't installed yet.

When you run `update`, iji checks the latest commit and rebuilds only if something changed. If the package is pinned to a version, it gets skipped.

Known limitations:

- Cargo `CARGO_TARGET_DIR` / custom `target-dir` isn't supported.
- Only executables are installed; libraries and other files are ignored.
- Make/CMake need install rules that put binaries in `<prefix>/bin` (CMake 3.15+). Make must honor `DESTDIR`/`PREFIX`; CMake must use relative destinations.
- Go builds may need network access; without a target, all `main` packages are installed.
- `install --force` can leave orphaned files that `update` and `remove` can't clean.

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

