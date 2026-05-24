//! Integration tests for Gitea API interactions.
//!
//! These tests verify that Knope can correctly:
//! - Create releases on a Gitea instance via the Release workflow
//! - Handle authentication errors gracefully
//!
//! All tests clean up after themselves by deleting any resources they create.

use std::{path::Path, process::Command, time::Duration};

use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
}

#[derive(Debug, Deserialize)]
struct RepoInfo {
    empty: bool,
}

struct GiteaEnv {
    token: String,
    host: String,
    owner: String,
    repo: String,
}

fn gitea_env() -> GiteaEnv {
    GiteaEnv {
        token: std::env::var("KNOPE_INTEGRATION_GITEA_TOKEN")
            .expect("KNOPE_INTEGRATION_GITEA_TOKEN must be set"),
        host: std::env::var("KNOPE_INTEGRATION_GITEA_HOST")
            .expect("KNOPE_INTEGRATION_GITEA_HOST must be set"),
        owner: std::env::var("KNOPE_INTEGRATION_GITEA_OWNER")
            .expect("KNOPE_INTEGRATION_GITEA_OWNER must be set"),
        repo: std::env::var("KNOPE_INTEGRATION_GITEA_REPO")
            .expect("KNOPE_INTEGRATION_GITEA_REPO must be set"),
    }
}

use super::integration_helpers::{push_branch, redact_url_credentials};

fn http_client() -> Client {
    Client::builder()
        .user_agent("Knope")
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build HTTP client")
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(dir)
        .output()
        .expect("Failed to run git command")
}

fn assert_git(dir: &Path, args: &[&str]) {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {} failed:\nstdout: {}\nstderr: {}",
        args.join(" "),
        redact_url_credentials(&String::from_utf8_lossy(&output.stdout)),
        redact_url_credentials(&String::from_utf8_lossy(&output.stderr))
    );
}

/// Add a git remote without including the URL in any panic message, so that
/// tokens embedded in the URL are not leaked into test logs.
fn set_git_remote(dir: &Path, remote_url: &str) {
    let output = Command::new("git")
        .args(["remote", "add", "origin", remote_url])
        .current_dir(dir)
        .output()
        .expect("Failed to run git remote add");
    assert!(output.status.success(), "git remote add failed");
}

/// Build a git remote URL for a Gitea instance with embedded token auth.
fn gitea_remote_url(token: &str, host: &str, owner: &str, repo: &str) -> String {
    let host_without_proto = host
        .strip_prefix("https://")
        .or_else(|| host.strip_prefix("http://"))
        .unwrap_or(host);
    format!("https://knope:{token}@{host_without_proto}/{owner}/{repo}.git")
}

fn api_base(host: &str, owner: &str, repo: &str) -> String {
    format!("{host}/api/v1/repos/{owner}/{repo}")
}

/// Set up a temporary directory with a Git repo, knope.toml, Cargo.toml, and CHANGELOG.md
/// configured for a Gitea release workflow.
///
/// The repo is pre-configured at version `0.1.0` so that the `Release` step can immediately
/// create a Gitea release without needing `PrepareRelease` or a `git push` inside knope.
/// The caller is responsible for pushing the branch to the remote before running knope.
fn setup_test_repo(
    token: &str,
    host: &str,
    owner: &str,
    repo: &str,
    branch: &str,
) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path();

    assert_git(path, &["init", "-b", branch]);
    assert_git(
        path,
        &["config", "user.email", "integration-test@knope.dev"],
    );
    assert_git(path, &["config", "user.name", "Knope Integration Test"]);

    let remote_url = gitea_remote_url(token, host, owner, repo);
    set_git_remote(path, &remote_url);

    // Workflow contains only the Release step: the repo is already at the right version,
    // so no PrepareRelease or git push is needed inside the knope workflow.
    let knope_toml = format!(
        r#"[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "Release"

[gitea]
owner = "{owner}"
repo = "{repo}"
host = "{host}"
"#
    );
    std::fs::write(path.join("knope.toml"), knope_toml).expect("Failed to write knope.toml");
    // Version is already at 0.1.0; the Release step will detect there is no v0.1.0 tag yet
    // and create a Gitea release pointing at the current HEAD commit.
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"integration-test\"\nversion = \"0.1.0\"\n",
    )
    .expect("Failed to write Cargo.toml");
    // Provide release notes for v0.1.0 so knope sends a non-empty body.
    // Without this, CreateReleaseInput sets generate_release_notes=true, which causes
    // Forgejo to attempt auto-generation and fail with "The target couldn't be found"
    // when there is no prior release to diff against.
    std::fs::write(
        path.join("CHANGELOG.md"),
        "## 0.1.0\n\n### Features\n\n- Initial release\n",
    )
    .expect("Failed to write CHANGELOG.md");

    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "-m", "chore: release"]);

    dir
}

async fn delete_release(client: &Client, token: &str, base: &str, release_id: u64) {
    let url = format!("{base}/releases/{release_id}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;
}

async fn delete_tag(client: &Client, token: &str, base: &str, tag: &str) {
    let url = format!("{base}/tags/{tag}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;
}

async fn delete_branch(client: &Client, token: &str, base: &str, branch: &str) {
    let url = format!("{base}/branches/{branch}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;
}

/// Return the full SHA of HEAD in the given repository directory.
fn get_head_sha(dir: &Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .expect("Failed to run git rev-parse HEAD");
    assert!(output.status.success(), "git rev-parse HEAD failed");
    String::from_utf8(output.stdout)
        .expect("Invalid UTF-8 in git output")
        .trim()
        .to_string()
}

/// Poll Forgejo until the repository is no longer marked as empty in its database.
///
/// When the test cleanup deletes the `integration-test-release` branch (which may
/// be the only branch), Forgejo marks the repository as `empty: true` in its
/// database. Re-pushing the branch updates the git pack files immediately, but the
/// database update (`is_empty = false`) may happen asynchronously via Forgejo's
/// internal queue.
///
/// This matters because the releases API route uses `ReferencesGitRepo()` (without
/// `allowEmpty=true`), which skips opening the git repository when `is_empty=true`,
/// leaving `ctx.Repo.GitRepo` as nil. This causes the release service to fail with
/// 404 "The target couldn't be found." In contrast, the tags API uses
/// `ReferencesGitRepo(true)` and is not affected by the `is_empty` flag.
///
/// We poll `GET /repos/{owner}/{repo}` until `"empty": false` before running
/// `knope release` to ensure the database has been updated.
async fn wait_for_repo_non_empty(client: &Client, token: &str, base: &str) {
    for attempt in 0..30u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        let resp = client
            .get(base)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .expect("Failed to call Gitea repo API");
        if resp.status().is_success() {
            if let Ok(info) = resp.json::<RepoInfo>().await {
                if !info.empty {
                    return;
                }
            }
        }
    }
    panic!("Timed out waiting for Forgejo repo to become non-empty");
}

/// Poll Forgejo until its git layer has fully indexed the pushed commit.
///
/// Even after `is_empty` is set to false, the git pack files on disk may not
/// yet be accessible to the release service. We probe by attempting to create
/// a lightweight temporary tag (`knope-probe`) at the HEAD SHA. A 201 response
/// confirms the git layer can resolve the SHA.
///
/// A 409 response means a stale probe tag from a previous run is still present
/// (our cleanup may have failed); we delete it and retry rather than treating
/// it as a readiness signal for the current SHA.
///
/// The probe tag is intentionally left alive after this function returns so
/// that the commit remains reachable from a git ref while `knope release` runs.
/// The caller is responsible for deleting it afterwards.
async fn wait_for_gitea_git_layer(client: &Client, token: &str, base: &str, head_sha: &str) {
    const PROBE_TAG: &str = "knope-probe";
    let url = format!("{base}/tags");

    for attempt in 0..20u32 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        let body = json!({
            "tag_name": PROBE_TAG,
            "target": head_sha,
            "message": ""
        });

        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&body)
            .send()
            .await
            .expect("Failed to call Gitea tags API");

        let status = resp.status();
        if status.as_u16() == 201 {
            // Created successfully at the current HEAD SHA → git layer is ready.
            // Probe tag is intentionally left alive; caller deletes it after knope release.
            return;
        }
        if status.as_u16() == 409 {
            // A stale probe tag from a previous run exists at an unknown SHA.
            // Delete it and retry to get a fresh 201 at the current HEAD SHA.
            delete_tag(client, token, base, PROBE_TAG).await;
            continue;
        }
        // Any other status (e.g. 404 "target not found") → not ready yet, keep retrying.
    }

    // Clean up if somehow the probe tag ended up created on the last attempt.
    delete_tag(client, token, base, PROBE_TAG).await;
    panic!(
        "Timed out waiting for Forgejo's git layer to index the pushed commit (SHA: {head_sha})"
    );
}

async fn cleanup_release_by_tag(client: &Client, token: &str, base: &str, tag: &str) {
    let url = format!("{base}/releases/tags/{tag}");
    if let Ok(resp) = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(release) = resp.json::<Release>().await {
                delete_release(client, token, base, release.id).await;
            }
        }
    }
    delete_tag(client, token, base, tag).await;
}

/// Test that `knope release` creates a Gitea release via the real API.
///
/// Sets up a git repository pre-configured at version `0.1.0`, pushes the branch to
/// the remote, runs `knope release` (Release step only), then verifies the release
/// was actually created on the Gitea instance.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_release_workflow() {
    let env = gitea_env();
    let client = http_client();
    let base = api_base(&env.host, &env.owner, &env.repo);
    let branch = "integration-test-release";
    let expected_tag = "v0.1.0";

    // Clean up any leftover resources from a previous failed run
    cleanup_release_by_tag(&client, &env.token, &base, expected_tag).await;
    delete_branch(&client, &env.token, &base, branch).await;
    delete_tag(&client, &env.token, &base, "knope-probe").await;

    let dir = setup_test_repo(&env.token, &env.host, &env.owner, &env.repo, branch);
    let path = dir.path();

    // Push the branch so the commit is sent to Forgejo.
    push_branch(path, branch);

    let head_sha = get_head_sha(path);

    // Step 1: Wait until Forgejo's database no longer considers the repo empty.
    // Deleting and re-pushing the branch may temporarily leave the repo marked
    // as empty in the DB (async update). The releases API skips opening the git
    // repo when is_empty=true, returning 404 "The target couldn't be found."
    wait_for_repo_non_empty(&client, &env.token, &base).await;

    // Step 2: Confirm the git layer (pack files) can resolve the pushed SHA.
    // The probe tag is left alive so the commit stays reachable from a ref
    // while knope release runs (prevents any potential GC race).
    wait_for_gitea_git_layer(&client, &env.token, &base, &head_sha).await;

    // Run knope release (Release step only — no PrepareRelease, no git push).
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITEA_TOKEN", &env.token)
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    // Delete the probe tag now that knope release has completed.
    delete_tag(&client, &env.token, &base, "knope-probe").await;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        cleanup_release_by_tag(&client, &env.token, &base, expected_tag).await;
        delete_branch(&client, &env.token, &base, branch).await;
        panic!("knope release failed:\nstdout: {stdout}\nstderr: {stderr}");
    }

    // Gitea may take a moment to finalise a new release; poll with retries.
    let url = format!("{base}/releases/tags/{expected_tag}");
    let mut release_opt = None;
    for _ in 0..5 {
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", env.token.as_str()))
            .send()
            .await
            .expect("Failed to fetch release");
        if resp.status().is_success() {
            release_opt = Some(
                resp.json::<Release>()
                    .await
                    .expect("Failed to deserialize release"),
            );
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let Some(release) = release_opt else {
        cleanup_release_by_tag(&client, &env.token, &base, expected_tag).await;
        delete_branch(&client, &env.token, &base, branch).await;
        panic!("Release {expected_tag} should exist on Gitea after retries");
    };
    assert_eq!(release.tag_name, expected_tag);

    // Cleanup
    delete_release(&client, &env.token, &base, release.id).await;
    delete_tag(&client, &env.token, &base, expected_tag).await;
    delete_branch(&client, &env.token, &base, branch).await;
}

/// Test that Knope handles authentication errors gracefully on Gitea.
///
/// Runs `knope release` with an invalid `GITEA_TOKEN` and verifies
/// the command fails with a non-zero exit code.
#[tokio::test]
#[ignore = "requires external service credentials"]
async fn gitea_error_bad_token() {
    let env = gitea_env();

    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let path = dir.path();

    assert_git(path, &["init"]);
    assert_git(
        path,
        &["config", "user.email", "integration-test@knope.dev"],
    );
    assert_git(path, &["config", "user.name", "Knope Integration Test"]);

    // Workflow with only the Release step — it will fail at the API call due to the bad token.
    let knope_toml = format!(
        r#"[package]
versioned_files = ["Cargo.toml"]
changelog = "CHANGELOG.md"

[[workflows]]
name = "release"

[[workflows.steps]]
type = "Release"

[gitea]
owner = "{owner}"
repo = "{repo}"
host = "{host}"
"#,
        owner = env.owner,
        repo = env.repo,
        host = env.host,
    );
    std::fs::write(path.join("knope.toml"), knope_toml).expect("Failed to write knope.toml");
    std::fs::write(
        path.join("Cargo.toml"),
        "[package]\nname = \"integration-test\"\nversion = \"0.1.0\"\n",
    )
    .expect("Failed to write Cargo.toml");
    std::fs::write(
        path.join("CHANGELOG.md"),
        "## 0.1.0\n\n### Features\n\n- Initial release\n",
    )
    .expect("Failed to write CHANGELOG.md");

    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "-m", "chore: release"]);

    // Run knope release with a bad token
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITEA_TOKEN", "bad-token-value")
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    assert!(
        !output.status.success(),
        "knope release should fail with a bad token"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.is_empty(),
        "Expected error output when using a bad token"
    );
}
