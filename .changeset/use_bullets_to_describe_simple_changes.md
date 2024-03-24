---
default: minor
---

# Use bullets to describe simple changes

The previous changelog & forge release format used headers for the summary of all changes, these entries were hard 
to follow for simple changes like this:

```markdown
### Features

#### A feature

#### Another header with no content in between?
```

Now, _simple_ changes are described with bullets at the _top_ of the section. More complex changes will come after 
any bullets, using the previous format:

```markdown
### Features

- A simple feature
- Another simple feature

#### A complex feature

Some details about that feature
```

Right now, a simple change is any change which comes from a conventional commit (whether from the commit summary or 
from a footer) _or_ a changeset with only a header in it. Here are three simple changes:

```
feat: A simple feature

Changelog-Note: A note entry
```

```markdown
---
default: minor
---

# A simple feature with no description
```

A complex change is any changeset which has content (not just empty lines) below the header.

PR #969 implemented #930. Thanks for the suggestion @ematipico!
