---
default: major
---

# Change changeset title level

The level of the title of a changeset no longer impacts the level of the release header in the changelog. To make this more obvious, changeset title are now level one headers by default. This is a breaking change because older versions of Knope will no longer properly handle the changesets from newer versions of Knope.
