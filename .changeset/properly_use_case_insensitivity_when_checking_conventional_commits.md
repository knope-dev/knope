---
knope: major
---

# Properly use case insensitivity when checking conventional commits

Per the [conventional commits spec](https://www.conventionalcommits.org/en/v1.0.0/#specification) all units of a 
conventional commit are case-insensitive.
Until now, Knope was treating commit footers and scopes as case-sensitive. This has been corrected, which may result 
in different behavior for some projects.
