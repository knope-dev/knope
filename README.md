# Knope

[![Discord](https://img.shields.io/discord/1191584005112467456.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/W75uRrBCEM)


A command line tool that happily completes the tasks which most developers find tedious.

## Example: Automating GitHub Actions Release

Got some conventional commits?

```
feat: A spicy feature
fix: Some sauce
```

And some changesets?

```
---
my-package: major
---

#### Big deal

You probably want to read this before upgrading 💜
```

Do you want to release this by hand? Knope! Here's a GitHub Actions workflow:

```yaml
name: Drop a new version

on: workflow_dispatch

jobs:
  create-release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0
          token: ${{ secrets.PAT }}
      - uses: knope-dev/action@v1 # Install Knope
        with:
          version: 0.7.4
      - run: knope release
        env:
          GITHUB_TOKEN: ${{ secrets.PAT }}
```

You get a GitHub release and a changelog, picking the [semantic version] based on the combination of [conventional commits] and [changesets].

```markdown
## 2.0.0

### Breaking Changes

#### Big deal

You probably want to read this before upgrading 💜

### Features

#### A spicy feature

### Fixes

#### Some sauce
```

Knope can do much more with some customization, [read the docs](https://knope.tech) for more info.

[conventional commits]: https://www.conventionalcommits.org
[semantic version]: https://semver.org
[changesets]: https://github.com/changesets/changesets
