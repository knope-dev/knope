---
default: major
---

#### Changelog entries now use 4th level headers instead of bullets

In order to support more detailed changelogs via [changesets](https://knope-dev.github.io/knope/config/step/PrepareRelease.html) (like the extra text you're seeing right now!) instead of each change entry being a single bullet under the appropriate category (e.g., `### Breaking Changes` above), it will be a fourth-level header (`####`). So, where _this_ changelog entry would have currently looked like this:

```markdown
### Breaking Changes

- Changelog entries now use 4th level headers instead of bullets
```

It now looks like what you're seeing:

```markdown
### Breaking Changes

#### Changelog entries now use 4th level headers instead of bullets

... recursion omitted
```
