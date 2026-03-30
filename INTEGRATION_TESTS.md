# Integration Tests

These tests exercise real HTTP calls against the GitHub and Gitea APIs to
validate that the `reqwest`-based HTTP client works correctly end-to-end. They
are **not** part of the default test suite and only run in a dedicated CI job
where the required secrets are available.

## Running locally

```bash
# Run all integration tests (requires all env vars below)
cargo test --test integration -- --ignored

# Run only GitHub tests
cargo test --test integration -- --ignored integration_github

# Run only Gitea tests
cargo test --test integration -- --ignored integration_gitea
```

## Required environment variables

### GitHub tests

| Variable                           | Description                                                    |
| ---------------------------------- | -------------------------------------------------------------- |
| `KNOPE_INTEGRATION_GITHUB_TOKEN`   | Personal access token with `repo` scope                        |
| `KNOPE_INTEGRATION_GITHUB_OWNER`   | Owner (user or org) of the test repository                     |
| `KNOPE_INTEGRATION_GITHUB_REPO`    | Name of the test repository                                    |

### Gitea tests

| Variable                          | Description                                                     |
| --------------------------------- | --------------------------------------------------------------- |
| `KNOPE_INTEGRATION_GITEA_TOKEN`   | Personal access token for the Gitea instance                    |
| `KNOPE_INTEGRATION_GITEA_HOST`    | Full URL of the Gitea instance (e.g. `https://codeberg.org`)   |
| `KNOPE_INTEGRATION_GITEA_OWNER`   | Owner (user or org) of the test repository                      |
| `KNOPE_INTEGRATION_GITEA_REPO`    | Name of the test repository                                     |

## Setting up test repositories

### GitHub

1. **Create a dedicated test repository** under a user or organization
   (e.g. `knope-dev/knope-integration-tests`). It can be public or private.
2. The repository must have a **`main` branch** with at least one commit
   (an initial commit with a README is sufficient).
3. **Create a fine-grained personal access token** (or classic token) with
   `repo` scope that has access to the test repository.
4. No webhooks, branch protection rules, or special settings are required.

### Gitea (e.g. Codeberg)

1. **Create an account** on the Gitea instance you want to test against
   (e.g. [Codeberg](https://codeberg.org)).
2. **Create a dedicated test repository** (e.g. `knope-integration-tests`).
   It should have a **`main` branch** with at least one commit.
3. **Create a personal access token** in your Gitea account settings.
   The token needs permissions to create/delete releases, branches, and
   pull requests.
4. No special repository settings are required.

### What the tests do

The tests create and immediately clean up the following resources:

- **Releases**: Created with a tag like `integration-test-release-v0.0.0`,
  then deleted (including the tag).
- **Pull requests**: A branch is created from `main`, a PR is opened and
  updated, then the PR is closed and the branch is deleted.
- **Asset uploads** (GitHub only): A small text file is uploaded to a
  draft release, then the entire release is deleted.

All test resources use names prefixed with `integration-test-` so they are
easy to identify if cleanup fails.

## CI setup

The integration tests run in `.github/workflows/integration_tests.yml`,
triggered by `workflow_dispatch` (manual run).

### Required repository configuration

1. **Repository variable** (Settings → Variables → New repository variable):
   - `INTEGRATION_TESTS_ENABLED` = `true`

2. **Repository secrets** (Settings → Secrets → New repository secret):

   For GitHub tests:
   - `INTEGRATION_GITHUB_TOKEN`
   - `INTEGRATION_GITHUB_OWNER`
   - `INTEGRATION_GITHUB_REPO`

   For Gitea tests:
   - `INTEGRATION_GITEA_TOKEN`
   - `INTEGRATION_GITEA_HOST`
   - `INTEGRATION_GITEA_OWNER`
   - `INTEGRATION_GITEA_REPO`

3. To run the tests, go to **Actions → Integration Tests → Run workflow**.
