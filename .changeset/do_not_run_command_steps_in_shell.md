---
default: major
---

# Don't run `Command` steps in shell

The `Command` step no longer attempts to run the command in a default shell for the detected operating system.
This fixes a compatibility issue with Windows.

If this change doesn't work for your workflow, please open an issue describing your need so we can fix it.

PR #919 closes issue #918. Thanks for reporting @alex-way!
