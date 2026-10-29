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

Building packages also needs the tool for whichever build system the repo uses (`cargo`, `go`, `cmake`, `meson`, or `make`). Custom builds (see below) need `bash`.

By default, iji puts package binaries in `~/.local/bin`. If that folder isn't in your `PATH` yet, add it:

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

Give iji a git URL:

```sh
iji install https://github.com/u/r
```

If you install the same repos often, give them short names in `iji.toml`:

```toml
[repos.pkgit]
url = "https://git.symlinx.net/pkgit"

[repos.mytool]
url = "https://github.com/u/mytool"
version = "v1.2"   # optional, same as `mytool@v1.2`
target = "mytool"  # optional, same as `mytool,mytool`
```

A bare package name looks up `[repos.<name>]`:

```sh
iji install pkgit
```

Repo keys must be valid package names. Letters, digits, `-`, `_`, and `.` are allowed, but a name can't start with `.`.

A version or target given on the command line wins over the repo entry:

```sh
iji install mytool@v2
```

The database stores the resolved URL, so `update` and `remove` keep working even if you remove the repo from the config later. A repo entry with a `version` counts as pinned, so `update` skips it.

You can also choose a version or a target for a URL:

```text
<git-url>[@version][,target]
```

```sh
iji install https://github.com/u/r@v1.2
iji install https://github.com/u/r@v1.2,mytool
```

The package name comes from the end of the URL, with `.git` removed. Add `@version` to install a tag or branch. Packages installed with a version stay on that version, so `update` leaves them alone.

To update or remove a package, use its name: the `[repos.<name>]` key for bare installs, or the last part of the URL for URL installs:

```sh
iji update r
iji remove r
```

A target is useful when a repo builds more than one thing. What it means depends on the build system:

| Build system | `target` |
| --- | --- |
| Cargo | Binary name (`--bin`) |
| Go | Package path, such as `cmd/foo` (defaults to `./...`) |
| Meson | Build target for `meson compile`; the install step still runs afterward |
| CMake | Build target (`--target`); the install step still runs afterward |
| Make | Goal for the first `make` run; `make install` still runs afterward |

For example, to build only the `cmd/foo` package in a Go repo:

```sh
iji install https://github.com/u/r@v1.2,cmd/foo
```

## Custom builds

When auto-detection doesn't fit, a `[repos.<name>]` entry can supply its own build with `run` (inline bash) or `script` (a bash file):

```toml
[repos.example]
url = "https://github.com/u/example"
run = """
meson setup build --prefix="$IJI_PREFIX"
meson compile -C build
meson install -C build --destdir "$IJI_STAGE"
"""

[repos.other]
url = "https://github.com/u/other"
script = "other.sh"   # relative to the iji config directory
```

The script runs in the cloned source directory and must install into a staging directory, not the real system. iji sets:

| Variable | Value |
| --- | --- |
| `IJI_SRC` | The source directory (also the working directory) |
| `IJI_STAGE` | The staging directory; iji empties it before each build |
| `IJI_PREFIX` | `/usr` |

After the script succeeds, iji copies the executables in `$IJI_STAGE/usr/bin` into your bin directory and records them, so `remove` deletes exactly those files. If the script fails, an existing install and its record are left untouched.

Rules:

- `run` and `script` are mutually exclusive, and `run` can't be empty. `run` is executed as `bash -euo pipefail -c`; `script` is executed as `bash <file>`, so the file sets its own flags.
- `script` must be a relative path without `..`.
- `target` can't be combined with `run` or `script`, in the config or on the command line.
- The commands come only from `iji.toml`, never from the repo.
- Only a bare name installed through `[repos]` uses the script. Installing by URL always auto-detects.
- Like other builds, only `usr/bin` is installed; other staged files are ignored.

`update` re-runs the script, so the `[repos.<name>]` entry (and the script file) must still exist. If it doesn't, `update` fails and leaves the install alone; it never falls back to auto-detection. Because `update` skips packages whose upstream commit hasn't changed, an edited script takes effect on the next upstream change or with `iji install --force <name>`.

Don't use `sudo` in scripts and don't run iji as root. Build as your user and install into `$IJI_STAGE`. iji doesn't install system software (kernel modules, services, anything that needs root); use your distro or the project's own installer for those.

## What works

iji is still pretty early, and it only runs on Unix. It detects the build system from the repo:

| Build system | Detected by | Tool needed |
| --- | --- | --- |
| Cargo | `Cargo.toml` | `cargo` |
| Go | `go.mod` | `go` |
| Meson | `meson.build` | `meson` (and `ninja`) |
| CMake | `CMakeLists.txt` | `cmake` |
| Make | `GNUmakefile`, `makefile`, or `Makefile` | `make` |

Detection happens in that order, unless the repo entry sets `run` or `script`. Go, Meson, CMake, and Make builds stage their executables in `usr/bin`, and iji copies those into your bin directory. Files a project installs elsewhere (libraries, data, man pages) aren't installed yet.

When you run `update`, iji checks the latest commit and rebuilds only if something changed. If the package is pinned to a version, it gets skipped.

Known limitations:

- Cargo `CARGO_TARGET_DIR` / custom `target-dir` isn't supported.
- Only executables are installed; libraries and other files are ignored.
- Make/CMake need install rules that put binaries in `<prefix>/bin` (CMake 3.15+). Make must honor `DESTDIR`/`PREFIX`; CMake must use relative destinations.
- Go builds may need network access; without a target, all `main` packages are installed.

## Configuration

Most people won't need a config file. iji reads `iji.toml` from its config directory: `$IJI_CONFIG_DIR/iji.toml` if that is set, otherwise the platform config directory with `iji` appended (usually `~/.config/iji/iji.toml`).

Alongside `[repos]`, you can choose where package binaries, repo checkouts, and build artifacts live:

```toml
[paths]
bin_dir = "~/.local/bin"
cache_dir = "~/.cache/iji"
source_dir = "~/.cache/iji/src"
```

`source_dir` is the root for repo checkouts: each package is cloned into its own folder. It defaults to `<cache_dir>/src`, usually `~/.cache/iji/src`, so the `iji` package would be cloned into `~/.cache/iji/src/iji`.

Changing `source_dir` only affects future fetches; existing checkouts stay where they are. When iji fetches a package, it replaces `source_dir/<package>`, so avoid pointing it at a directory containing hand-managed clones with the same names.

Paths must be absolute or start with `~`. Environment variables win over `iji.toml`, which wins over the defaults. `data_dir` and `config_dir` can't be set in the file; use the environment variables for those. The `[repos.*]` and `[paths]` tables can appear in any order. Unknown keys are rejected, so typos fail instead of being silently ignored.

| Variable | What it does |
| --- | --- |
| `IJI_BIN_DIR` | Where package binaries are installed |
| `IJI_DATA_DIR` | Where `installed.toml` is stored |
| `IJI_CACHE_DIR` | Where build artifacts are staged; `source_dir` defaults to `<cache_dir>/src` |
| `IJI_SOURCE_DIR` | Where repo checkouts are cloned |
| `IJI_CONFIG_DIR` | Where `iji.toml` is read from |

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

