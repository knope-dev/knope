---
title: Jira
---

Configuration needed to use steps that reference Jira issues.

## Example

```toml
# knope.toml

[jira]
url = "https://mysite.atlassian.net"
project = "PRJ"  # where an example issue would be PRJ-123
```

The first time you use a step which requires this config,
you will be prompted to generate a Jira API token so Knope can perform actions on your behalf.
