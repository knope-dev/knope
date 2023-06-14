---
default: minor
---

### Added support for changesets

Leveraging the new [changesets crate](https://github.com/knope-dev/changesets), Knope now supports [changesets](https://github.com/changesets/changesets)! In short, you can run `knope document-change` (if using default workflows) or add the new [`CreateChangeFile`] step to a workflow to generate a new Markdown file in the `.changeset` directory. You can then fill in any additional details below the generated header in the generated Markdown file. The next time the `PrepareRelease` step runs (e.g., in the default `knope release` workflow), all change files will be consumed to help generate a new version and changelog (along with any conventional commits).

For additional details, see:

- [`PrepareRelease` step](https://knope-dev.github.io/knope/config/step/PrepareRelease.html)
- [`CreateChangeFile` step](https://knope-dev.github.io/knope/config/step/CreateChangeFile.html)
- [Packages (for how you can customize changelog sections)](https://knope-dev.github.io/knope/config/packages.html)
