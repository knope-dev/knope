---
knope: minor
versioning: minor
---

# Add first-class Deno release support to Knope

#1577 by @jeckhart

* Detect Deno projects, workspaces, and nested packages so they can be released with Knope
* Reuse the deno_config and deno_lockfile crates to stay current with maintained Deno config and lockfile formats
* Extend release pipeline so Deno projects get version bumps and changelogs consistent with Node.js projects
* Updated Concept docs for Change, Changeset, Package
* Updated Reference docs for Packages and Default Config
* Add tests for deno based projects and workspaces

