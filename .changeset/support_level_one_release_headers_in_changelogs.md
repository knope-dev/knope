---
default: minor
---

#### Support level-one release headers in changelogs

If the last release in a changelog file has a level-one header instead of Knope's default of level-two, new releases will be created with level-one headers as well. Sections will then be level two instead of level three. Note that changesets will still be _authored_ the same way (level 4 headers) to avoid having to parse the changelog when authoring a changeset—but they will be _combined_ into the changelog correctly.
