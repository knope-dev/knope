---
title: Customizing changelogs
---

You don't have to settle for the built-in changelog sections,
there are a number of ways to customize generated changelogs.
Each top-level heading on this page is a different way to customize; you can skip right to the most interesting one for you.

Before we get started, here is how a [version in a changelog](/reference/concepts/changelog#versions) looks by default:

```markdown
## 2.0.0 (2023-10-31)

### Breaking changes

#### A breaking change

Some details about that change

### Features

#### A new feature

Some details about that feature

### Fixes

#### A bug fix

Some details about that fix

### Notes

#### A note

Some details about that note
```

The order of these sections is fixed, but any section without changes will be omitted.

## Changing the header level

By default, the heading of a version is `##`, each section is `###`, and each change is `####`.
The _relative_ level of those sections is fixed,
but you can change each version to be a top-level heading (`#`)
by modifying the last version in the changelog to be that level.
Knope looks for the previous version to determine the level of the next version.

:::caution
Knope expects versions in the changelog to look roughly how it would generate them,
specifically it _must_ start with a valid [semantic version](/reference/concepts/semantic-versioning).
If you have a different format for your pre-existing headers, you'll need to update the _latest_ version to Knope's format.
:::

```markdown
# 2.0.0 (2023-10-31)

## Breaking changes

### A breaking change

# 1.0.0

This was edited to be a top-level heading
```

## Adding additional sections

You can use the `extra_changelog_sections` config option to add additional sections to a changelog.
This is per-package, so if you have multiple packages, you'll need to customize each changelog.

```toml title="knope.toml"
[package]
extra_changelog_sections = [
    { name = "Security", footers = ["Security"], types = ["sec"] }
]
```

You can add as many sections as you want, they will appear in order _after_ the built-in sections.
Each section can be added to from any number of [conventional commit footers] and [changeset types].
The semantic version impact of any custom changes is `patch`.

## Overriding built-in sections

The built-in sections, as described at the top of this page, can be overridden.

:::caution
The default order is recommended, as it organizes changes from most-important (for consumers) to least-important.
Overridden sections always appear after non-overridden sections in the order they are defined.
So, if you're going to override any of the first three sections,
it's recommended you also override any sections that should appear below them.
:::

Here's some config that would override all the built-in sections,
_only_ changing their name (not their order or sources):

```toml title="knope.toml"
[package]
extra_changelog_sections = [
    { types = ["major"], name = "❗️Breaking ❗" },
    { types = ["minor"], name = "🚀 Features" },
    { types = ["patch"], name = "🐛 Fixes" },
    { footers = ["Changelog-Note"], name = "📝 Notes" },
]
```

## Adding to `Notes` from more sources

The built-in `Notes` section comes from any [conventional commit footers](/reference/concepts/conventional-commits#footers) named `Changelog-Note`.
You can override this section in the config file to add more sources:

```toml title="knope.toml"
[package]
extra_changelog_sections = [
    { name = "Notes", footers = ["Changelog-Note"], types = ["note"] }
]
```

Now, when running a [`CreateChangeFile`] step (e.g., with `knope document-change`), the `note` type will be available:

```text
? What type of change is this?
  major
  minor
  patch
> note
[↑↓ to move, enter to select, type to filter]
```
