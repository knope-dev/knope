---
title: Packages
---

A package is a piece of software you release.
It has a single version number, though that version number can be in many places.
For example, Knope is a package,
which at the time of writing has the version number `0.13.0` in a single file, `Cargo.toml`.
To release Knope on NPM as well, we'd add a `package.json` file with the same version number.

To split out some of Knope's functionality into a Rust crate that others could consume,
say `knope-changelogs`, _that_ crate would be a _separate_ package with its own version number.

To decide whether two versioned files are part of _one_ package or if they should be _separate_ packages,
ask yourself whether _every_ change that affects one always affects both.

## Version

The current version of the package is the version number in _all_ versioned files.
If there is any inconsistency, that's an error.
If there are no versioned files, the package's version is the last [release]'s Git tag.
If there also is no valid Git tag, the package doesn't have a version (which could be an error sometimes).

[release]: /reference/concepts/release
