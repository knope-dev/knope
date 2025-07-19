---
title: Release notes
---

Release notes are information about a particular version of a [package]. The goal is to provide users of your project 
with important information about _why_ they should want to update to the new version (like new features, better performance, or bug fixes)
as well as any challenges they might face when upgrading (like breaking changes).

Most projects have their release notes in two difference places:

1. A markdown file called a [changelog] which lists the entire version history for a package in one place
2. A tagged [release] on a [forge] (like GitHub) where users can be notified of new versions

It's important for both of these locations (if both are used) to be consistent and accurate, so they're generated 
from the same sources and in _almost_ the same format.

:::tip
You can [customize your release notes in a number of ways][customize].
:::


## Examples

Here's what release notes might look like in a changelog:

```markdown
## 1.2.3 (2022-12-03)

### Fixes

- A simple fix
- Another

#### A more complex fix

Some details about that.
```

And that same release on GitHub:

```markdown
## Fixes

- A simple fix
- Another

### A more complex fix

Some details about that.
```

The main difference is that because a changelog lists all versions in one file, the headers for each section are smaller.
The version number and date also go in a separate title attribute for a forge, not in the Markdown content.

## Sections

Every change is assigned to a section within the changelog, like `## Fixes` above. These sections are determined by the 
change type of a [change file] or the type or footer of a [conventional commit]. Sections have a fixed order that they 
always appear in, but the section itself only appears if it contains at least one change.

The default sections are:

| Section          | Changeset Type | Commit Type | Commit Footer     |
|------------------|----------------|-------------|-------------------|
| Breaking Changes | `major`        | `!`         | `BREAKING CHANGE` |
| Features         | `minor`        | `feat:`     |                   |
| Fixes            | `patch`        | `fix:`      |                   |
| Notes            |                |             | `Changelog-Note`  |

:::tip

You can [customize] these sections.

:::

## Simple vs complex changes

Knope divides each change section into simple and complex changes.
Changes that are descibed by only a single sentence are simple changes. All changes which come from a [conventional commit] are simple changes.
Additionally, any [change file] with only a header in is a simple change.
Knope includes simple changes as bullets at the top of a section.

For example, `feat: a simple feature` and a change file containing only `# Another simple feature` below its frontmatter will appear as

```markdown
## Features

- a simple feature
- Another simple feature
```

Complex changes are [change files][change file] that have content below their header. Each complex change gets its own sub-section with
a header. Headers are auto-adjusted to the appropriate level for the changelog or release. For example, this change file:

```markdown
---
default: minor
---

# A more complicated feature

With a more complicated description.
```

Would appear after the simple changes like this:

```markdown
## Features

- a simple feature
- Another simple feature

### A more complicated feature

With a more complicated description.
```

:::tip

You can [customize] how each change appears, but they will always be listed with simple first, followed by complex 
within their section.

:::

[package]: /reference/concepts/package
[changelog]: /reference/concepts/changelog
[release]: /reference/concepts/release
[forge]: /reference/concepts/forge
[change file]: /reference/concepts/change-file
[conventional commit]: /reference/concepts/conventional-commits
[customize]: /recipes/customizing-release-notes