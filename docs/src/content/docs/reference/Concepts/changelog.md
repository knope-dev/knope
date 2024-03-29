---
title: Changelog
---

In Knope, a changelog is a Markdown file which documents every change **relevant to users** for a [package].
The format of a changelog looks like this:

```markdown
# Some optional title for the changelog

Some details about how versioning works, how to read the changelog, etc.

## 2.0.0 (2023-02-01)

### Breaking changes

- A breaking change

## 1.2.3 (2022-12-03)

### Fixes

- A simple fix
- Another

#### A more complex fix

Some details about that.
```

:::tip
You can [customize your changelogs in a number of ways](/recipes/customizing-changelogs).
:::

Changelogs have a number of parts:

## Title (optional)

Most, but not all Markdown files start with a title. This should probably be `# Changelog` or `# My Package Changelog`.

## Introduction (optional)

It's a good idea to explain to your consumers how changes work for this package.
At a minimum, describe what's considered a breaking change for _your_ package,
as this looks different for different packages.

## Versions

Each version starts with a heading (default level 2, `##`) with the version number and the release data, formatted like this:

```markdown
## 1.2.3 (2022-12-03)
```

Following the version heading, there's at least one section heading (one level below the version heading) grouping changes by type,
for example "Breaking changes" or "Features."

```markdown
### Breaking changes
```

Following the section heading, there's at least one change.
[Simple changes](#simple-vs-complex-changes) will appear as bullets immediately under the section heading:

```markdown
- A very easy to describe change
- Another as well
```

Next, [complex changes](#simple-vs-complex-changes) will each get a heading (two levels below the version heading):

```markdown
#### A breaking change
```

After each change heading, there's a description of the change. Put all together, a version looks something like this:

```markdown
## 1.2.3 (2022-12-03)

### Breaking changes

#### A breaking change

This is a description of the breaking change.

### Fixes

- A simple fix
- Another simple fix

#### A more complicated fix

Some details about the fixing
```

Knope sorts versions from newest to oldest,
so the most recent version is near the top of the changelog right after the optional title and introduction.

## Simple vs complex changes

Knope divides each change section into simple and complex changes.
Changes that are descibed by only a single sentence are simple changes. All changes which come from [conventional commits](/reference/concepts/conventional-commits) are simple changes.
Additionally, any changeset with only a header in is a simple change.
Knope includes simple changes as bullets at the top of a section.

Complex changes are changesets that have content below their header. Each complex change gets its own sub-section with
a header.

[package]: /reference/concepts/package
