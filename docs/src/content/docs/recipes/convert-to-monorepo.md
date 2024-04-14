---
title: "Convert a single-package repository to a monorepo"
---

:::tip

If you're going from one `Cargo.toml` file to a Cargo workspace, and you don't have anything custom in your `[package]` section,
you may be able to use Knope's [Cargo workspace support](/reference/default-config#cargo-workspaces).

:::

## Step 1: Reorganize your repository

Before adding in a new package, you should move the existing package to wherever it will live in the monorepo.

For example, when Knope switched to a monorepo,
the code for `knope` moved from the root of the repository to `crates/knope`.
What's important from Knope's perspective is
where the [`versioned_files`](/reference/config-file/packages#versioned_files) and [`changelog`](/reference/config-file/packages#changelog) are.

## Step 2: Update `knope.toml`

Replace the `package` section with a `packages.<package-name>` section in `knope.toml`.
This section works identically to the `package` section,
so the only thing that needs to change is the path to `versioned_files` and `changelog`.

Again, using Knope's own transformation as an example:

```diff lang="tomml" title="knope.toml"
- [package]
+ [packages.knope]
- versioned_files = ["Cargo.toml"]
+ versioned_files = ["crates/knope/Cargo.toml"]
changelog = "CHANGELOG.md"
```

In this case, `changelog` stayed in the same place, but `Cargo.toml` moved.

### Step 3: Create a monorepo-style tag

Tagging [works differently](/reference/concepts/release/#git-tags) in a monorepo. In order for Knope to find the
correct tags and commits going forward, you'll need to create a fresh tag using the new syntax.

```bash "<package-name>"
LAST_TAG=$(git describe --tags $(git rev-list --tags --max-count=1))
git tag <package-name>/$LAST_TAG $LAST_TAG
```

Be sure to replace `<package-name>` with the same thing you put in `knope.toml`.

### Step 4: Add in new packages

Now that you have a monorepo set up, you can add in new packages as needed. Create a new `[packages.<package-name>]`
section in `knope.toml` for each new package. Make sure to also add an initial version tag to each new package,
like `package-name/v0.0.0` so that Knope won't add conventional commits from before the package's creation to its
first release.
