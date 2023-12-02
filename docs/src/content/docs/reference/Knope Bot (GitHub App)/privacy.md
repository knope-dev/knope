---
title: "Data Collection and Privacy"
---

:::caution

Knope Bot doesn't currently have a way to notify you when things on this page change.
If you want to be notified of changes to this policy, please [open an issue](https://github.com/knope-dev/knope/issues)
with a suggestion for how you'd like to be notified.

:::

Knope Bot collects and stores data for a few reasons:

1. **Caching** for improved performance (and to obey GitHub's rate limits). This data is discarded **as soon as it's no longer useful**. Some examples:
   1. Information about an installation (generally, the relationship between an organization and this app) is discarded when the app is uninstalled.
   2. Pull request details are deleted when the pull request is closed or the associated installation is uninstalled.
2. Delaying webhook processing to meet GitHub's API limits. **Entire webhook payloads** may be stored **until they're processed** (generally within a few minutes).
3. **Logs** to aid with debugging, deleted within **2 weeks**.
4. **Error reports and performance metrics** sent to Sentry, which retains them for **90 days**. This is based on the best answer in their forums and doesn't appear to be configurable.

Knope Bot does _not_ collect, store, or share data for any marketing or advertising purposes.
