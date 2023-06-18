---
default: minor
---

#### Remove duplicate version from GitHub release name

Release notes in GitHub releases used to copy the entire section of the changelog, including the version number. Because the name of the release also includes the version, you'd see the version twice, like:

```markdown
# 1.0.0

## 1.0.0

... notes here
```

Now, that second `## 1.0.0` is omitted from the body of the release.
