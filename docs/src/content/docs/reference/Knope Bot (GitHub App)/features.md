---
title: Features
---

Knope Bot is a GitHub App that performs common Knope tasks automatically without complex GitHub Actions workflows.
You can install the bot from [GitHub Marketplace](https://github.com/marketplace/knope-bot).
Only install it for the repositories you want to enforce documentation on (as it will do this by default).

## Enforcing documentation

The bot keeps an up-to-date check on all non-draft pull requests to verify that they're documented,
with at least one [change file] or [conventional commit].

:::tip

Want more control? Open an issue in [the main Knope repo](https://github.com/knope-dev/knope/issues)!

:::

### Double-checking documentation

When a check from Knope Bot passes or fails, the details will include an explanation of _why_, so that you can
double-check Knope's work.

:::note

For efficiency, Knope Bot stops processing as soon as a check passes. So, if a pull request is documented in more than
one way, only the first will appear in the check details.

:::

### Change files

If a pull request has at least one [change file], it will pass the check:

![Screenshot of the details of a passing check which state that the pull request has a change file,
along with the path to that file](./passing-check-change-file.png)

### Conventional commits

There are three ways to merge a pull request: merge commits, rebasing, and squashing.
_Every_ method which is enabled for a repository must result in at least one [conventional commit]
for the pull request to be considered documented via conventional commits.
In the details of the check,
you will see an bullet point for each merge method explaining why that _method_ passed or failed
(independent of the others).

#### Pull request title

In two circumstances, the title of the pull request will become a commit message—in these cases, a check can pass
by checking only the pull request title:

1. Squash merging when _either_ there is more than one commit in the pull request _or_ the repository is configured to _always_ use the pull request title as the commit title. By default, if there is only one commit, GitHub will use that commit when squash merging (effectively rebasing).
2. Merge commits when the repository is configured to use the pull request title as the commit title. By default, GitHub uses a generic merge commit message, not the pull request title.

#### Commit messages

For all the merge methods enabled on the repository and not covered by the pull request title,
the bot will check the commit messages included in the pull request.
If Knope Bot can parse _any_ of the commits in the pull request as a
[conventional commit] (even if they wouldn't change the next version or changelog), the check will pass.

### Disabling this check

Add the following to a `knope.toml` file in the root of the repository to disable this check:

```toml
[bot.checks]
enabled = false
```

## Creating change files

When a check fails, the details of that check will contain instructions for creating a changeset:

![Screenshot of the details of a failing Knope Bot check. There are three buttons at the top: "Major (Breaking),"
"Minor (Feature)," and "Patch (Fix)," Below those buttons are instructions for using them, followed by instructions
for creating a changeset manually.](./failing-check-details.png)

For pull requests that aren't from forks,
project members will see buttons to create [change files][change file] directly for the three main [change types](/reference/concepts/semantic-versioning).
Clicking one of those buttons will cause the bot to commit the change file directly to the pull request branch.

For anyone who can't see those buttons (or anyone who wants more control over the change documentation), there is a
link to using Knope's CLI.
There's also an example Markdown snippet that contributors can copy into a change file manually.

## Previewing and creating releases

Add the following to a `knope.toml` file in the root of the repository to enable releases:

```toml
[bot.releases]
enabled = true
```

When releases are enabled, Knope Bot will keep an up-to-date pull request with a preview of your next release. This pull request comes from a branch called
`knope/release`. Merging the pull request will create a GitHub release.

### Creating draft releases

When Knope Bot creates a release for a package,
if that package has any [assets configured](/reference/config-file/packages/#assets),
it will create a draft release.
Knope Bot does not use the content of the assets config for anything,
merely setting it to a value triggers the draft behavior.

See the ["Uploading assets to draft releases"](/recipes/uploading-assets-to-draft-releases) recipe for more information.

[change file]: /reference/concepts/change-file
[conventional commit]: /reference/concepts/conventional-commits
