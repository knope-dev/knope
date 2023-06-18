---
default: minor
---

#### Added dates to version titles

There are now release dates in both changelogs and version names on GitHub. This probably won't break your releases, but you will have a different format for release notes which could be jarring. The date is in the format `YYYY-MM-DD` and will always be based on UTC time (so if you do a release late at night on the east coast of the United States, the date will be the next day).

Previously, the changelog entry title would look like this:

```markdown
## 1.0.0
```

And now it will look like this:

```markdown
## 1.0.0 (2023-06-10)
```
