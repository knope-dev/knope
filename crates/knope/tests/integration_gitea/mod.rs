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

#[derive(Debug, Deserialize)]
struct Release {
    id: u64,
    tag_name: String,
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

fn http_client() -> Client {
    Client::builder()
        .user_agent("Knope")
        .build()
        .expect("Failed to build HTTP client")
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
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
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
    std::fs::write(path.join("CHANGELOG.md"), "").expect("Failed to write CHANGELOG.md");

    assert_git(path, &["add", "."]);
    assert_git(path, &["commit", "-m", "chore: release"]);

    dir
}

async fn delete_release(client: &Client, token: &str, base: &str, release_id: u64) {
    let url = format!("{base}/releases/{release_id}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .send()
        .await;
}

async fn delete_tag(client: &Client, token: &str, base: &str, tag: &str) {
    let url = format!("{base}/tags/{tag}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .send()
        .await;
}

async fn delete_branch(client: &Client, token: &str, base: &str, branch: &str) {
    let url = format!("{base}/branches/{branch}");
    let _ = client
        .delete(&url)
        .header("Authorization", format!("token {token}"))
        .send()
        .await;
}

async fn cleanup_release_by_tag(client: &Client, token: &str, base: &str, tag: &str) {
    let url = format!("{base}/releases/tags/{tag}");
    if let Ok(resp) = client
        .get(&url)
        .header("Authorization", format!("token {token}"))
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

    let dir = setup_test_repo(&env.token, &env.host, &env.owner, &env.repo, branch);
    let path = dir.path();

    // Push the branch so knope can resolve the HEAD commit SHA when creating the release.
    assert_git(
        path,
        &["push", "--set-upstream", "origin", branch, "--force"],
    );

    // Run knope release (Release step only — no PrepareRelease, no git push).
    let output = Command::new(env!("CARGO_BIN_EXE_knope"))
        .current_dir(path)
        .env("GITEA_TOKEN", &env.token)
        .args(["release"])
        .output()
        .expect("Failed to run knope");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
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
            .header("Authorization", format!("token {}", env.token.as_str()))
            .send()
            .await
            .expect("Failed to fetch release");
        if resp.status().is_success() {
            release_opt =
                Some(resp.json::<Release>().await.expect("Failed to deserialize release"));
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    let release = match release_opt {
        Some(r) => r,
        None => {
            cleanup_release_by_tag(&client, &env.token, &base, expected_tag).await;
            delete_branch(&client, &env.token, &base, branch).await;
            panic!("Release {expected_tag} should exist on Gitea after retries");
        }
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
    std::fs::write(path.join("CHANGELOG.md"), "").expect("Failed to write CHANGELOG.md");

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
