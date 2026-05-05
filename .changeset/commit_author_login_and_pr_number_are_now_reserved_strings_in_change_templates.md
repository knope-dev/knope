---
knope: major
---

# `$commit_author_login` and `$pr_number` are now reserved strings in change templates

If you previously had these literal strings in `[release_notes.change_templates]`, they will now be treated like the new
variales for looking up GitHub info.
