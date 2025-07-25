---
title: Change
---

A change, in the context of Knope releases, is a single thing that is different from one release to another which is relevant to a user.
Changes can be documented using either a [change file] or a 
[conventional commit](/reference/concepts/conventional-commits).
A Git commit may contain no changes, one change, or even many changes.
Each change appears in the project's [release notes](/reference/concepts/release-notes) and affects a package's 
[version](/reference/concepts/semantic-versioning).

## Example changes

### A breaking change

We added support for `package-lock.json` files to Knope, including for projects which have no `knope.toml` file.
While this is a feature, more importantly it's a breaking change because Knope is now automatically updating files
by default which it didn't before. That context is worth noting explicitly for users with more than a simple bullet point,
so we document this with a [change file].

```markdown
---
knope: major
knope-versioning: minor
---

# Support `package-lock.json` files

`package-lock.json` files are [now supported](https://knope.tech/reference/config-file/packages/#package-lockjson)
as `versioned_files` both for single packages and dependencies (in monorepos).

These files will be auto-detected and updated if using the default (no `[package]` or `[packages]`) config, so
this is a breaking change for those users.
```

### A feature

We enhanced the `--verbose` flag to be more verbose. Not a ton of detail to add, so we can document this with a simple 
[conventional commit].

```
feat(knope): Print each step before it runs when `--verbose` is set (#1399)
```

### A bug fix

Here we have a complex bug fix that requires some explaining.
It's important to let users know what happened here in case they were relying on the old behavior! 

````markdown
---
knope: patch
---

# Fix multiple versioned files with same path

Previously, if you referenced the same file multiple times in versioned_files, only the first instance would apply.
Now, if you reference the same path multiple times, each instance will be processed sequentially.

Consider a mono-repo where every package should be versioned in lockstep:

```toml
[package]
versioned_files = [
  "package.json",
  { path = "package.json", dependency = "aDependency" },  # Before this fix, this was ignored
  "aDependency/package.json"
]
```

This use-case is now supported!
````

### A note

We updated the minimum supported Rust version of Knope.
This isn't expected to impact most users, but _may_ impact anyone redistributing Knope, so we include it as a simple 
`Note` using a conventional commit footer:

```
chore: Update Rust

Changelog-Note: Update to Rust edition 2024 and MSRV 1.85
```

## Example non-changes

Only changes that are relevant to users should be documented and result in a version increase.
Documenting hidden changes clutters up the release notes, making it less likely that users will see what _is_ important.

### Updating an internal dependency

Unless updating the dependency changes the behavior for the user in some way, they won't care that it was updated!

```
chore: Updated TOML parser to 0.9
```

Some cases where the user _might_ care:

1. You are publishing a library, and updating the dependency allows the user to clean up their own dependency tree.
    Include these changes as a `Note`.
2. The dependency contained a security issue _which impacted the project_. Note the relevant issue and its fix under `Fixes`.
3. Updating the dependency changes the behavior of your program. Note the relevant changes as `Breaking Changes`

### Refactoring some code

A lot of Git commits move around or restructure code without intentionally changing the behavior of the application.
In this case, you shouldn't add any notes about this refactor.
If the refactor was part of implementing some fix, performance improvement, or new feature, then document it!

### Fixing typos in the docs

There are definitely times you'll want to inform users that a new docs article exists, but _most_ of the time docs 
changes impact _future_ users, not current users (who are reading your release notes).

### Changes to your CI/CD process

Things which only impact developers of your project and not the _users_ of the project don't belong in the changelog.
You should definitely notify developers of significant change, but that should use a channel other than releases.

Times when this _might_ matter to users:

1. Your package is now distributed in a new location
2. You added some new signing or other security mechanism to artifacts
3. You had to publish a new version for an internal reason, but there are no user-facing changes

[change file]: /reference/concepts/change-file