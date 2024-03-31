# ParadeDB Cargo Dev Tool

## Installation

The first time you install `cargo-paradedb`, you should navigate to the `cargo-paradedb` crate and run:

```sh
cargo run install
```

After this first-time installation, you can run `cargo paradedb install` from anywhere and Cargo will globally re-install `cargo-paradedb` re-compiled with the latest code changes from your source folder.

If you don't want to install globally, you can always just `cargo run` from the `cargo-paradedb` crate folder.

### Installing From Git Url

In containers or cloud instances, it's useful to be able to install globally with a single command:

```sh
cargo install --git https://github.com/paradedb/paradedb.git cargo-paradedb
```

This will install the tool for use as `cargo paradedb` without having to clone the repository first. You can also specify a branch:

```sh
cargo install \
    --git https://github.com/paradedb/paradedb.git \
    --branch new-feature-branch \
    cargo-paradedb
```
