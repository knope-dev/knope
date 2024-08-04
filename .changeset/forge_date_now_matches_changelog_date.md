---
knope: major
---

# Forge date now matches CHANGELOG date

If you prepare a release and generate a changelog Markdown file in one workflow, then create a forge release in a
separate workflow, the forge release date will now match the changelog date (if any). Previously, the forge release got
the current date (at the time of running the workflow).
