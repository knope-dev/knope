---
default: patch
---

#### Only consider prereleases newer than the last stable

This fixes a regression in the previous version of Knope where _all_ prereleases would be considered, rather than just those tagged after the latest stable version.
