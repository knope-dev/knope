---
default: major
---

#### Reworked Go versioning

In order to support running `Release` in a separate workflow from `PrepareRelease` and to fix a bug relating to Go module tags (when in a subdirectory), Knope will now store the full package version in a comment in the `go.mod` file and use that version as the source of truth for the package version. This has a couple of implications:

1. If you already have a comment on the `module` line in `go.mod` which matches the correct format, Knope may not be able to determine the version correctly.
2. If you have a comment on that line which does _not_ match the format, it will be erased the next time Knope bumps the version.

In either case, the solution is to erase or move that comment. Here is the syntax that Knope is looking for:

`module {ModulePath} // v{Version}`

If that comment does not exist, Knope will revert to looking for the latest relevant Git tag instead to _determine_ the version, but will still write the comment to the `go.mod` file when bumping the version.
