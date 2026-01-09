# Issue: GitHub/Gitea Releases Created on Wrong Branch

## Problem Summary

When `knope release` creates a GitHub or Gitea release via their APIs, it does not pass the `target_commitish` parameter. This causes the tag to be created on the repository's **default branch** (typically `main`) rather than the current HEAD commit.

This breaks workflows where:

1. A temporary branch is created with version updates
2. The workflow checks out that branch
3. `knope release` is run expecting the tag to point to the commit on that branch
4. Instead, the tag ends up pointing to `main`, missing all the version updates

## Solution

Pass `target_commitish` with the current HEAD commit SHA when calling the GitHub/Gitea release APIs.

---

## Files to Modify

### 1. `crates/knope/src/integrations/git.rs`

**Add a new public function** to get the current HEAD commit SHA. The pattern already exists in `create_tag()` at lines 441-442:

```rust
let head = repo.head()?;
let head_commit = head.peel_to_commit()?;
```

Create a new function like:

```rust
pub(crate) fn get_head_commit_sha() -> Result<String, Error> {
    let repo = Repository::open(current_dir().map_err(ErrorKind::CurrentDirectory)?)?;
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    Ok(head_commit.id().to_string())
}
```

---

### 2. `crates/knope/src/integrations/mod.rs` (lines 16-42)

**Add `target_commitish` field** to the `CreateReleaseInput` struct:

```rust
#[derive(Serialize)]
struct CreateReleaseInput<'a> {
    tag_name: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
    prerelease: bool,
    generate_release_notes: bool,
    draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_commitish: Option<&'a str>,  // ADD THIS
}
```

**Update the `new()` constructor** (lines 30-42) to accept and set `target_commitish`:

```rust
impl<'a> CreateReleaseInput<'a> {
    fn new(
        tag_name: &'a str,
        name: &'a str,
        body: &'a str,
        prerelease: bool,
        draft: bool,
        target_commitish: Option<&'a str>,  // ADD THIS PARAMETER
    ) -> Self {
        let body = if body.is_empty() { None } else { Some(body) };
        Self {
            generate_release_notes: body.is_none(),
            tag_name,
            name,
            body,
            prerelease,
            draft,
            target_commitish,  // ADD THIS
        }
    }
}
```

---

### 3. `crates/knope/src/integrations/github/create_release.rs`

**At line 26-27**, update the call to `CreateReleaseInput::new()`:

- Get the HEAD commit SHA using the new function from git.rs
- Pass it to `CreateReleaseInput::new()`

Current code (line 26-27):

```rust
let github_release =
    CreateReleaseInput::new(tag_name, name, body, prerelease, assets.is_some());
```

Should become something like:

```rust
let target_commitish = git::get_head_commit_sha().ok();
let github_release =
    CreateReleaseInput::new(tag_name, name, body, prerelease, assets.is_some(), target_commitish.as_deref());
```

**Update dry run logging** in `github_release_dry_run()` (lines 130-160) to include target commitish info.

---

### 4. `crates/knope/src/integrations/gitea/create_release.rs`

**At line 20**, update the call to `CreateReleaseInput::new()`:

Current code:

```rust
let gitea_release = CreateReleaseInput::new(tag_name, name, body, prerelease, false);
```

Should become something like:

```rust
let target_commitish = git::get_head_commit_sha().ok();
let gitea_release = CreateReleaseInput::new(tag_name, name, body, prerelease, false, target_commitish.as_deref());
```

**Update dry run logging** in `gitea_release_dry_run()` (lines 52-67) to include target commitish info.

---

## Implementation Notes

1. **Error handling**: Using `.ok()` on `get_head_commit_sha()` makes the SHA optional - if we can't get it for some reason, the release will still work (falling back to GitHub/Gitea's default behavior). This is a graceful degradation.

2. **Both GitHub and Gitea support this**: The `target_commitish` parameter works identically in both APIs.

3. **Import needed**: The GitHub and Gitea create_release.rs files will need to import the git module to access `get_head_commit_sha()`.

4. **API Reference**:
   - GitHub: https://docs.github.com/en/rest/releases/releases#create-a-release
   - The parameter "specifies the commitish value that determines where the Git tag is created from. Can be any branch or commit SHA. Unused if the Git tag already exists."

---

## Post-Implementation

Consider adding a test if appropriate.
