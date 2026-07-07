---
versioning: major
config: major
---

# New APIs for path-based commit routing

To support `track_paths`:

- `knope-versioning`: the conventional commit `Commit` struct has a new public `files` field listing the paths each commit changed (breaking for struct-literal construction).
- `knope-config`: `Package` has new `track_paths` and `paths` fields (breaking for struct-literal construction).
