---
title: GitHub Actions workflow dispatch
---

With this recipe, you can trigger the entire release process manually
by either clicking a button in GitHub Actions or by using the GitHub CLI.
Once that trigger occurs:

1. Knope calculates the new version of the project (using [`PrepareRelease`]) and updates versioned files and changelogs.
2. Knope commits the changes back to the branch and pushes.
3. GitHub Actions builds assets from the new commit.
4. Knope creates a release on GitHub with the new version, changelog, and assets.

:::hint
You should also check out the [Pull Request Releases](/recipes/1-preview-releases-with-pull-requests) recipe,
which is similar, but includes release previews via pull requests.
:::

:::note
All the examples in this recipe are for a project with a single Rust binary to release—
you'll need to adapt some specifics to your use-case.
:::

Here's the GitHub Actions workflow file:

```yaml
name: Release

on: workflow_dispatch

jobs:
  prepare-release:
    runs-on: ubuntu-latest
    outputs:
      sha: ${{ steps.commit.outputs.sha }}
    steps:
      - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 # v5.0.0
        name: Fetch entire history (for conventional commits)
        with:
          fetch-depth: 0
          token: ${{ secrets.PAT }}
      - name: Configure Git
        run: |
          git config --global user.name GitHub Actions
          git config user.email github-actions@github.com
      - name: Install Knope
        uses: knope-dev/action@v2.1.0
        with:
          version: 0.21.7
      - run: knope prepare-release --verbose
        name: Update versioned files and changelog
      - name: Store commit
        id: commit
        run: echo "sha=$(git rev-parse HEAD)" >> $GITHUB_OUTPUT

  build-artifacts:
    needs: prepare-release
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
      package_name: # TODO: put the name of your package here
    runs-on: ${{ matrix.os }}
    name: ${{ matrix.target }}

    steps:
      - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 # v5.0.0
        with:
          ref: ${{ needs.prepare-release.outputs.sha }}

      - name: Install host target
        run: rustup target add ${{ matrix.target }}

      - name: Install musl-tools
        if: ${{ matrix.target == 'x86_64-unknown-linux-musl' }}
        run: sudo apt-get install -y musl-tools

      - uses: Swatinem/rust-cache@v2.8.2

      - name: Build
        run: cargo build --release --target ${{ matrix.target }}

      - name: Create Archive Folder
        run: mkdir ${{ env.archive_name }}

      - name: Copy Unix Artifact
        if: ${{ matrix.os != 'windows-latest' }}
        run: cp target/${{ matrix.target }}/release/${{ env.package_name }} ${{ env.archive_name }}

      - name: Copy Windows Artifact
        if: ${{ matrix.os == 'windows-latest' }}
        run: cp target/${{ matrix.target }}/release/${{ env.package_name }}.exe ${{ env.archive_name }}

      - name: Create Tar Archive
        run: tar -czf ${{ env.archive_name }}.tgz ${{ env.archive_name }}

      - name: Upload Artifact
        uses: actions/upload-artifact@v5.0.0
        with:
          name: ${{ matrix.target }}
          path: ${{ env.archive_name }}.tgz
          if-no-files-found: error

  release:
    needs: [build-artifacts, prepare-release]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@08c6903cd8c0fde910a37f88322edcfb5dd907a8 # v5.0.0
        with:
          ref: ${{ needs.prepare-release.outputs.sha }}
      - uses: actions/download-artifact@v6.0.0
        with:
          path: artifacts
          merge-multiple: true
      - name: Install the latest Knope
        uses: knope-dev/action@v2.1.0
        with:
          version: 0.21.7
      - run: knope release --verbose
        env:
          GITHUB_TOKEN: ${{ secrets.PAT }}
```

There are three jobs here:

1. `prepare-release` runs the `prepare-release` Knope workflow and saves the new commit as an output for use later.
2. `build-artifacts` builds the assets for the release from the new commit that `prepare-release` created.
3. `release` runs the `release` Knope workflow which creates the GitHub Release.

Throughout, there's use of a `${{ secrets.PAT }}`, this is a GitHub Token with write permissions to "contents" which
must be stored in GitHub Actions secrets.
For the minimum-possible required privileges,
you should [create a fine-grained access token] with read/write to "contents" for only this repo.

Here's a Knope config which enables this GitHub workflow to work:

```toml
[package]
versioned_files = ["Cargo.toml", "Cargo.lock"]
changelog = "CHANGELOG.md"
assets = "artifacts/*"

[[workflows]]
name = "prepare-release"

[[workflows.steps]]
type = "PrepareRelease"

[[workflows.steps]]
type = "Command"
command = "git commit -m \"chore: prepare release $version\""

[[workflows.steps]]
type = "Command"
command = "git push"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "Release"

[[workflows]]
name = "document-change"

[[workflows.steps]]
type = "CreateChangeFile"

[github]
owner = "knope-dev"
repo = "knope"
```

There's a single `[package]`, but this pattern should also work for multi-package setups, just make sure all your assets are ready at the same time.
In this case, there are two versioned files (`Cargo.toml` and `Cargo.lock`), the changelog (`CHANGELOG.md`), and all
assets (four, in the example GitHub workflow).

There are two relevant workflows here, the third (`document-change`) is for creating changesets during development. `prepare-release` starts by running the [`PrepareRelease`] step, which does the work of updating `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` based
on any conventional commits or changesets.
Knope then runs a command to commit the changes and push them back to the current branch (note that using the `$version` variable isn't supported for multi-package setups at this time). Once this workflow runs, the project is ready to build assets.

When ready, GitHub Actions calls into the `release` workflow which runs a single step: [`Release`].
This will compare the latest stable tagged release to the version in `Cargo.toml` (or any other `versioned_files`)
and create releases as needed by parsing the contents of `CHANGELOG.md` for the release's body.
The release is initially created as a draft, then Knope uploads assets before publishing the release
(so your subscribers won't be notified until it's ready).

[`PrepareRelease`]: /reference/config-file/steps/prepare-release
[create a fine-grained access token]: https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-fine-grained-personal-access-token
[`Release`]: /reference/config-file/steps/release
