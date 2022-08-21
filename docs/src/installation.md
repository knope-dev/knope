# Installation

## Install via [`cargo-binstall`] (Recommended)

Knope is built in such a way that [`cargo-binstall`] can install it by downloading a binary artifact for the most popular platforms.

1. Install [`cargo-binstall`] using your preferred method.
2. Run `cargo-binstall knope` to install Knope.

```admonish info
Is your platform not supported yet? Please contribute it by [opening a pull request](https://github.com/knope-dev/knope/pulls).
```

## Install via GitHub Action (Recommended)

If using GitHub Actions, the easiest way to install Knope is via [this action](https://github.com/marketplace/actions/install-knope).

## Download a Binary Manually

We automatically build binaries for some platforms which can be found on the [Releases](https://github.com/knope-dev/knope/releases) page.

## Install via Cargo

Knope is written in Rust and published on [crates.io](https://crates.io/crates/knope) which means it can therefore be built from source by:

1. Installing cargo via [Rustup]
2. Running `cargo install knope`

```admonish warning
Building Knope can be quite slow, if possible, it's recommended to download a prebuilt binary instead.
```

## Build from Source

1. Install the current Rust stable toolchain via [Rustup]
2. Clone [the GitHub repo](https://github.com/knope-dev/knope/)
3. `cargo install --path .` in the cloned directory

## Other

Have another method you'd prefer to use to install Knope? Let us know by [opening a GitHub issue](https://github.com/knope-dev/knope/issues).

[rustup]: https://rustup.rs
[`cargo-binstall`]: https://github.com/ryankurte/cargo-binstall
