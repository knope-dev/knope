---
title: Uploading assets to draft releases
---

If you need to upload assets to a release,
you likely want to create that release as a draft and only publish it once
you've uploaded the assets.

This recipe will show you how to configure Knope Bot to create draft releases
and provide a sample GitHub Actions workflow to upload and publish.

First, indicate to Knope Bot that there are assets to upload by adding the
`assets` config option to your `knope.toml` file:

```toml title="knope.toml"
[package]
# versioned_files and changelog config here
assets = "marker"

[bot.releases]
enabled = true
```

When you merge the release pull request from Knope Bot, it will create the
release as a draft instead of a published release.

:::tip

This also works for monorepos with multiple packages. Only packages with
`aasets` set are set to draft.

:::

Here's a complete example workflow, which is broken down step-by-step below:

```yaml title=".github/workflows/release.yml"
on:
  pull_request:
    types: [closed]
    branches: [main]
  workflow_dispatch:

jobs:
  get-tag:
    if: (github.head_ref == 'knope/release' && github.event.pull_request.merged == true) || github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4.2.2
      - run: echo "tag_name=$(gh release list --json 'isDraft,tagName' --jq '.[] | select(.isDraft) | .tagName')" >> $GITHUB_OUTPUT
        env:
          GH_TOKEN: ${{ github.token }}
        id: get-tag
    outputs:
      tag_name: ${{ steps.get-tag.outputs.tag_name }}
  build-artifacts:
    needs: [get-tag]
    if: needs.get-tag.outputs.tag_name != ''
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4.2.2
      - name: Replace Me with actual artifact creation
        run: echo "example artifact" >> artifact.txt
      - name: Upload Artifact
        uses: actions/upload-artifact@v4.6.1
        with:
          name: Example
          path: artifact.txt
          if-no-files-found: error

  release:
    needs: [build-artifacts, get-tag]
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4.2.2
      - uses: actions/download-artifact@v4.2.1
        with:
          path: artifacts
          merge-multiple: true
      - name: Upload artifacts to release
        run: |
          cd artifacts
          gh release upload ${{ needs.get-tag.outputs.tag_name }} *
          gh release edit ${{ needs.get-tag.outputs.tag_name }} --draft=false --latest
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

The whole workflow is triggered either manually
(in case something fails and needs to be retried)
or when the release pull request merges.
You can't detect a draft release with the `on.release` event.

```yaml
on:
  pull_request:
    types: [closed]
    branches: [main]
  workflow_dispatch:
```

The first job "get-tag" then determines what the GitHub release is that needs
to be edited.
This job only runs if the pull request was a _release_ pull request,
and only if the pull request merged (not just closed).

```yaml
get-tag:
  if: (github.head_ref == 'knope/release' && github.event.pull_request.merged == true) || github.event_name == 'workflow_dispatch'
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4.2.2
    - run: echo "tag_name=$(gh release list --json 'isDraft,tagName' --jq '.[] | select(.isDraft) | .tagName')" >> $GITHUB_OUTPUT
      env:
        GH_TOKEN: ${{ github.token }}
      id: get-tag
  outputs:
    tag_name: ${{ steps.get-tag.outputs.tag_name }}
```

:::caution

If you have multiple packages which all create draft releases, you'll need to
filter further.

:::

Next is a placeholder job for actually building the artifacts,
which you'll need to replace with your own logic.
This is a separate job so you can "fan out" with a matrix and build multiple
artifacts in parallel.

```yaml
build-artifacts:
  needs: [get-tag]
  if: needs.get-tag.outputs.tag_name != ''
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4.2.2
    - name: Replace Me with actual artifact creation
      run: echo "example artifact" >> artifact.txt
    - name: Upload Artifact
      uses: actions/upload-artifact@v4.6.1
      with:
        name: Example
        path: artifact.txt
        if-no-files-found: error
```

The final job downloads all artifacts uploaded in previous jobs and uploads them
to the release. It then sets the release as a non-draft and as the latest release.

```yaml
release:
  needs: [build-artifacts, get-tag]
  runs-on: ubuntu-latest
  permissions:
    contents: write
  steps:
    - uses: actions/checkout@v4.2.2
    - uses: actions/download-artifact@v4.2.1
      with:
        path: artifacts
        merge-multiple: true
    - name: Upload artifacts to release
      run: |
        cd artifacts
        gh release upload ${{ needs.get-tag.outputs.tag_name }} *
        gh release edit ${{ needs.get-tag.outputs.tag_name }} --draft=false --latest
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```
