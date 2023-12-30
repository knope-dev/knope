---
title: "Preview releases with pull requests"
---

This recipe always keeps an open pull request which previews the changes the Knope will include in the next release. This pull request will let you see the next version, the changes to versioned files, and the changelog. When you merge that pull request, Knope will create a new release with the changes from the pull request.

This recipe requires a custom `knope.toml` file and two GitHub Actions workflows.

## `knope.toml`

Each section below is separate for easier explanation, but all these TOML snippets exist in the same file.

### `[package]`

```toml
[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"
```

This first piece defines the package.
`Cargo.toml` is both the source of the package's current version and a place that Knope should put new version numbers.
You can add more `versioned_files` (for example, if you also released this as a Python package with `pyproject.toml`).
`CHANGELOG.md` is where Knope should describe changes in the source code—this is in _addition_ to GitHub releases.

:::caution
You can't use this recipe as-is with multiple packages due to limitations on [variables].
You'll either need to change any references to those variables or use the [workflow dispatch recipe].
:::

### `[[package.assets]]`

```toml
[[package.assets]]
path = "artifacts/knope-x86_64-unknown-linux-musl.tgz"

[[package.assets]]
path = "artifacts/knope-x86_64-pc-windows-msvc.tgz"

[[package.assets]]
path = "artifacts/knope-x86_64-apple-darwin.tgz"

[[package.assets]]
path = "artifacts/knope-aarch64-apple-darwin.tgz"
```

`package.assets` defines a list of files to upload to GitHub releases. You can also include `name` to change the name of the uploaded artifact.
It defaults to the last component of the path (for example, `knope-x86_64-unknown-linux-musl.tgz`).

### `prepare-release` workflow

```toml
[[workflows]]
name = "prepare-release"

[[workflows.steps]]
type = "Command"
command = "git switch -c release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "git commit -m \"chore: prepare release $version\" && git push --force --set-upstream origin release"

[workflows.steps.variables]
"$version" = "Version"

[[workflows.steps]]
type = "CreatePullRequest"
base = "main"

[workflows.steps.title]
template = "chore: prepare release $version"
variables = { "$version" = "Version" }

[workflows.steps.body]
template = "This PR was created by Knope. Merging it will create a new release\n\n$changelog"
variables = { "$changelog" = "ChangelogEntry" }
```

The first workflow has a `name` of `prepare-release`,
so `knope prepare-release` will execute it (the GitHub Actions workflow will contain this command).
First, it creates a new branch from the current one called `release`.
Next, it runs the [`PrepareRelease`] step, which updates the package based on the changes made since the last release.
It also stages all those changes with Git (like `git add`).

Next, the workflow commits the changes that [`PrepareRelease`] made—things like:

- Updating the version in `Cargo.toml`
- Adding a new section to `CHANGELOG.md` with the latest release notes
- Deleting any changesets

The workflow then pushes the commit to the `release` branch,
using the `--force` flag in this case because the history of that branch isn't important.

:::tip
You could use a variable to name the branch based on the version instead of erasing earlier versions.
:::

The [`CreatePullRequest`] step then creates a pull request from the current branch
(`release`) to the specified base branch (`main`).
It uses string templates containing [variables] to set the title and body,
in this case, the title includes the new `Version` and the body includes the new `ChangelogEntry`.

The pull request that this creates looks something like this:

![Pull Request Preview](./pull_request_preview.png)

### `release` workflow

```toml
[[workflows]]
name = "release"

[[workflows.steps]]
type = "Release"
```

The `release` workflow is a single [`Release`] step—this creates a GitHub release for the latest version
(if it doesn't already exist) and uploads any [assets](#packageassets).
In this case, it'll create a release for whatever the `prepare-release` workflow prepared earlier.
GitHub Actions will run this workflow whenever someone merges the pull request (created by `prepare-release`).

### `[github]`

The last piece is to tell Knope which GitHub repo to use for creating pull requests and releases.
You must substitute your own values here:

```toml
[github]
owner = "knope-dev"
repo = "knope"
```

## `prepare_release.yml`

There are two GitHub Actions workflows for this recipe—the first one goes in `.github/workflows/prepare_release.yml`
and it creates a fresh release preview pull request on every push to the `main` branch:

```yaml title=".github/workflows/prepare_release.yml"
on:
  push:
    branches: [main]
name: Create Release PR
jobs:
  prepare-release:
    if: "!contains(github.event.head_commit.message, 'chore: prepare release')" # Skip merges from releases
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
          token: ${{ secrets.PAT }}
      - name: Configure Git
        run: |
          git config --global user.name GitHub Actions
          git config user.email github-actions@github.com
      - uses: knope-dev/action@v2.0.0
        with:
          version: 0.13.0
      - run: knope prepare-release --verbose
        env:
          GITHUB_TOKEN: ${{ secrets.PAT }}
        continue-on-error: true
```

:::caution
This workflow runs by default on _every_ push to main, that includes when a release PR merges!
There is an `if:` clause here in the first job that skips it if the commit message looks like the ones create by the [`prepare-release` workflow](#prepare-release-workflow). If you change that message, you'll need to update this `if:` clause as well.
:::

The steps here:

1. Check out the _entire_ history of the repo (so that [`PrepareRelease`] can use tags and conventional commits to pick the next version). This requires a [personal access token] with permission to **read** the **contents** of the repo.
2. Configure Git so that the job can commit changes (within Knope's `prepare-release` workflow)
3. Install Knope
4. Run [the `prepare-release` workflow described earlier](#prepare-release-workflow). _This_ requires a [personal access token] with permission to **write** the **pull requests** of the repo.

:::note
The `continue-on-error` attribute means even if this step fails, the workflow will pass.
This is because the workflow runs on every push to `main`, but shouldn't fail when there's nothing to release.
However, the workflow also won't fail if there are real errors from Knope.
You may want to instead use the [`allow_empty` option](/reference/Config File/Step/prepare-release.md#options) in
`knope.toml` and split the rest of the steps into a second workflow.
Then, you can use some scripting in GitHub Actions to skip the rest of the workflow if there's nothing to release.
:::

In this example, the same [personal access token] is in both steps, but you could use separate ones if you wanted to.

## `release.yml`

Now that Knope is creating pull requests every push to `main`,
it needs to automatically release those changes when a pull request merges.
This is the job of the `release` workflow, which goes in `.github/workflows/release.yml`.

:::caution
YAML is sensitive to space and easy to mess up copy/pasting—so you should copy the whole file at _the end_, not the individual pieces.
:::

To start off, this workflow must only run
when release preview pull requests merge—there are several pieces of config that handle this.
First:

```yaml title="./github/workflows/release.yml"
on:
  pull_request:
    types: [closed]
    branches: [main]
```

Will cause GitHub Actions to only trigger the workflow when a pull request which targets `main` closes.
Then, in the _first_ job, an `if` narrows that down further to only release preview pull requests,
and only when they _merge_ (not close for other reasons):

```yaml
if: github.head_ref == 'release' && github.event.pull_request.merged == true
```

For Knope's own workflows, this first job is `build-artifacts`,
which builds the [package assets](#packageassets) that Knope will upload when releasing.
Skipping on past that job (since it probably will be different for you), the net one is the `release` job:

```yaml
release:
  needs: [build-artifacts]
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/download-artifact@v4.1.0
      with:
        path: artifacts
        merge-multiple: true
    - uses: knope-dev/action@v2.0.0
      with:
        version: 0.13.0
    - run: knope release
      env:
        GITHUB_TOKEN: ${{ secrets.PAT }}
```

The `release` job follows these steps:

1. Check out the repo at the commit that the pull request merged
2. Download the artifacts from the `build-artifacts` job
3. Install Knope
4. Run [the `release` workflow described earlier](#release-workflow). This requires a [personal access token] with permission to **write** the **contents** of the repo.

Finally, Knope's workflow publishes to crates.io—meaning the whole workflow looks like this:

```yaml
name: Release

on:
  pull_request:
    types: [closed]
    branches: [main]

jobs:
  build-artifacts:
    if: github.head_ref == 'release' && github.event.pull_request.merged == true
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest
    env:
      archive_name: artifact

    runs-on: ${{ matrix.os }}
    name: ${{ matrix.target }}

    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - name: Install host target
        run: rustup target add ${{ matrix.target }}

      - name: Install musl-tools
        if: ${{ matrix.target == 'x86_64-unknown-linux-musl' }}
        run: sudo apt-get install -y musl-tools

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Set Archive Name (Non-Windows)
        id: archive
        run: echo "archive_name=knope-${{ matrix.target }}" >> $GITHUB_ENV

      - name: Set Archive Name (Windows)
        if: ${{ matrix.os == 'windows-latest' }}
        run: echo "archive_name=knope-${{ matrix.target }}" | Out-File -FilePath $Env:GITHUB_ENV -Encoding utf8 -Append

      - name: Create Archive Folder
        run: mkdir ${{ env.archive_name }}

      - name: Copy Unix Artifact
        if: ${{ matrix.os != 'windows-latest' }}
        run: cp target/${{ matrix.target }}/release/knope ${{ env.archive_name }}

      - name: Upload Artifact
        uses: actions/upload-artifact@v4.0.0
        with:
          name: ${{ matrix.target }}
          path: ${{ env.archive_name }}.tgz
          if-no-files-found: error

  release:
    needs: [build-artifacts]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4.1.0
        with:
          path: artifacts
          merge-multiple: true
      - uses: knope-dev/action@v2.0.0
        with:
          version: 0.13.0
      - run: knope release
        env:
          GITHUB_TOKEN: ${{ secrets.PAT }}

  publish-crate:
    needs: [release]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: Swatinem/rust-cache@v2
      - uses: katyo/publish-crates@v2
        with:
          registry-token: ${{ secrets.CARGO_TOKEN }}
```

:::tip
[Use Renovate](https://github.com/knope-dev/action#updating-with-renovate) to keep those versions of Knope up to date.
:::

## Conclusion

Just to summarize, this recipe describes a process that:

1. Automatically creates a pull request in GitHub every time a new commit is pushed to `main`. That pull request contains a preview of the next release.
2. Automatically releases the package every time a release preview's pull request is merged.

[variables]: /reference/Config File/variables.md
[workflow dispatch workflow]: ./workflow_dispatch.md
[`PrepareRelease`]: /reference/Config File/Steps/prepare-release.md
[`CreatePullRequest`]: ../config/step/CreatePullRequest.md
[`Release`]: ../config/step/Release.md
[`CreateChangeFile`]: ../config/step/CreateChangeFile.md
[personal access token]: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-fine-grained-personal-access-token
