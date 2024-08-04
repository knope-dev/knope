---
knope: major
---

# Match scope-filtering behavior to docs

The docs state, in regard to a `package.scopes` config, "if not defined, Knope will consider all scopes."

This is the intended behavior, but wasn't true until now. The actual behavior, for multi-package repos, was that if
_any_ package had scopes defined, _all_ would start filtering scopes.

This has been corrected, packages are now more independent in their scope filtering behavior.
